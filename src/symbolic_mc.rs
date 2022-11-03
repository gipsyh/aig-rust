use crate::{Aig, AigEdge, AigNodeId};
use std::{assert_matches::assert_matches, mem::swap};

pub static mut TOTAL_SIMAND: usize = 0;
pub static mut TOTAL_SIMAND_SAT_INSERT: usize = 0;
pub static mut TOTAL_SIMAND_NOSAT_INSERT: usize = 0;
pub static mut TOTAL_RESIM: usize = 0;
pub static mut TOTAL_BUG: usize = 0;
pub static mut TOTAL_ADD_PATTERN: usize = 0;
pub static mut TOTAL_FRAIG_ADD_SAT: usize = 0;
pub static mut TOTAL_FE_MERGE_NODE: usize = 0;
pub static mut TOTAL_STASH_GET: usize = 0;
struct EliminateOrder {
    inputs: Vec<AigNodeId>,
}

impl EliminateOrder {
    fn new(inputs: Vec<AigNodeId>) -> Self {
        Self { inputs }
    }

    fn get_node(&mut self, aig: &Aig, observes: &[AigEdge]) -> Option<(AigNodeId, usize)> {
        let fanin = aig.fanin_logic_cone(observes);
        if self.inputs.is_empty() {
            return None;
        }
        let expect: Vec<usize> = self
            .inputs
            .iter()
            .map(|input| aig.calculate_expect_size(*input, &fanin))
            .collect();
        let mut min_now = expect[0];
        let mut ret = 0;
        for (i, e) in expect.iter().enumerate().skip(1) {
            if *e < min_now {
                min_now = *e;
                ret = i;
            }
        }
        Some((self.inputs.remove(ret), min_now))
    }

    fn cleanup_redundant(&mut self, nodes_map: &[Option<AigNodeId>]) {
        for input in self.inputs.iter_mut() {
            *input = nodes_map[*input].unwrap();
        }
    }
}

impl Aig {
    fn calculate_expect_size(&self, input: AigNodeId, ob_cone: &[bool]) -> usize {
        let fanout = self.fanout_logic_cone(input.into());
        let mut ret = 0;
        for i in 0..self.num_nodes() {
            if ob_cone[i] && fanout[i] {
                ret += 1;
            }
        }
        ret
    }
}

impl Aig {
    pub fn symbolic_mc_back(&mut self) -> bool {
        if self.bads.is_empty() {
            return true;
        }
        let init = self.latch_init_equation();
        let bads = self.bads.clone();
        let mut bad = self.bads[0];
        for b in &bads[1..] {
            bad = self.new_or_node(bad, *b);
        }
        let mut eliminate = Vec::new();
        for i in &self.inputs {
            eliminate.push(*i);
        }
        let (mut latch_map, transition) = self.transfer_latch_outputs_into_pinputs();
        for (x, y) in &mut latch_map {
            eliminate.push(*x);
            swap(x, y)
        }
        let mut frontier = bad;
        let mut deep = 0;
        loop {
            deep += 1;
            dbg!(deep, self.num_nodes());
            if self.sat_solver.solve(&[init, frontier]).is_some() {
                return false;
            }
            frontier = self.migrate_logic(&latch_map, frontier);
            let mut equation = self.new_and_node(frontier, transition);
            for iid in &eliminate {
                assert_matches!(self.nodes[*iid].typ, crate::AigNodeType::PrimeInput);
                equation = self.eliminate_input(*iid, vec![equation])[0];
                dbg!(self.num_nodes());
            }
            frontier = equation;
            let bad_new = self.new_or_node(bad, frontier);
            if bad != bad_new {
                bad = bad_new
            } else {
                dbg!(deep);
                return true;
            }
        }
    }

    pub fn symbolic_mc(&mut self) -> bool {
        if self.bads.is_empty() {
            if !self.outputs.is_empty() {
                self.bads.push(self.outputs[0]);
            } else {
                return true;
            }
        }
        let mut reach = self.latch_init_equation();
        let mut frontier = reach;
        let mut inputs = self.inputs.clone();
        for l in self.latchs.iter() {
            inputs.push(l.input);
        }
        let (mut latch_map, mut transition) = self.transfer_latch_outputs_into_pinputs();
        let mut bad = self.bads[0];
        for badid in 1..self.bads.len() {
            bad = self.new_or_node(bad, self.bads[badid]);
        }
        let mut deep = -1;
        loop {
            deep += 1;
            println!("deep {} begin, num nodes: {}", deep, self.num_nodes());
            dbg!(unsafe { TOTAL_BUG });
            if self.sat_solver.solve(&[bad, frontier]).is_some() {
                return false;
            }
            let mut equation = self.new_and_node(frontier, transition);
            let mut eliminate_order = EliminateOrder::new(inputs.clone());
            while let Some((mut enode, expect)) = eliminate_order.get_node(self, &[equation]) {
                assert!(self.nodes[enode].is_prime_input());
                if expect > 100 {
                    let nodes_map = self.cleanup_redundant(&mut [
                        &mut frontier,
                        &mut reach,
                        &mut transition,
                        &mut bad,
                        &mut equation,
                    ]);
                    eliminate_order.cleanup_redundant(&nodes_map);
                    enode = nodes_map[enode].unwrap();
                    for input in &mut inputs {
                        *input = nodes_map[*input].unwrap();
                    }
                    for (x, y) in &mut latch_map {
                        *x = nodes_map[*x].unwrap();
                        *y = nodes_map[*y].unwrap();
                    }
                    println!("after cleanup: {}", self.num_nodes());
                }
                equation = self.eliminate_input(enode, vec![equation])[0];
                dbg!(self.num_nodes());
                dbg!(self.fraig.as_ref().unwrap().nword());
            }
            frontier = self.migrate_logic(&latch_map, equation);
            let reach_new = self.new_or_node(reach, frontier);
            frontier = self.new_and_node(frontier, !reach);
            if reach != reach_new {
                reach = reach_new
            } else {
                dbg!(deep);
                dbg!(unsafe { TOTAL_SIMAND });
                dbg!(unsafe { TOTAL_SIMAND_NOSAT_INSERT });
                dbg!(unsafe { TOTAL_SIMAND_SAT_INSERT });
                dbg!(unsafe { TOTAL_RESIM });
                dbg!(unsafe { TOTAL_BUG });
                dbg!(unsafe { TOTAL_ADD_PATTERN });
                dbg!(unsafe { TOTAL_FRAIG_ADD_SAT });
                dbg!(unsafe { TOTAL_STASH_GET });
                dbg!(self.fraig.as_ref().unwrap().nword());
                return true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate test;

    use crate::Aig;
    #[test]
    fn test1() {
        let mut aig =
            Aig::from_file("/root/MC-Benchmark/examples/counter/10bit/counter.aag").unwrap();
        println!("{}", aig);
        // aig.fraig();
        dbg!(aig.symbolic_mc_back());
    }

    #[test]
    fn test2() {
        let mut aig = Aig::from_file("./aigs/counter-3bit.aag").unwrap();
        println!("{}", aig);
        // aig.fraig();
        dbg!(aig.symbolic_mc_back());
    }

    #[test]
    fn test3() {
        let mut aig = Aig::from_file(
            "/root/MC-Benchmark/hwmcc20/aig/2019/goel/crafted/paper_v3/paper_v3.aag",
        )
        .unwrap();
        println!("{}", aig);
        // aig.fraig();
        println!("{}", aig);
        dbg!(aig.symbolic_mc());
    }
}
