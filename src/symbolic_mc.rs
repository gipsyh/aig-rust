use crate::{Aig, AigEdge, AigNodeId};
use std::{assert_matches::assert_matches, mem::swap};

struct EliminateOrder {
    inputs: Vec<AigNodeId>,
}

impl EliminateOrder {
    fn new(inputs: Vec<AigNodeId>) -> Self {
        Self { inputs }
    }

    fn get_node(&mut self, aig: &Aig, observes: &[AigEdge]) -> Option<AigNodeId> {
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
        for i in 1..expect.len() {
            if expect[i] < min_now {
                min_now = expect[i];
                ret = i;
            }
        }
        Some(self.inputs.remove(ret))
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
        let bads = self.bads.clone();
        for b in &bads[1..] {
            bad = self.new_or_node(bad, *b);
        }
        let mut deep = 0;
        loop {
            deep += 1;
            dbg!(deep, self.num_nodes());
            if self.sat_solver.solve(&[bad, frontier]).is_some() {
                return false;
            }
            let mut equation = self.new_and_node(frontier, transition);
            let mut eliminate_order = EliminateOrder::new(inputs.clone());
            while let Some(mut enode) = eliminate_order.get_node(self, &[equation]) {
                assert_matches!(
                    self.nodes[enode].typ,
                    crate::AigNodeType::PrimeInput
                );
                {
                    let nodes_map =
                        self.cleanup_redundant(&[frontier, reach, transition, bad, equation]);
                    reach.set_nodeid(nodes_map[reach.node_id()].unwrap());
                    frontier.set_nodeid(nodes_map[frontier.node_id()].unwrap());
                    transition.set_nodeid(nodes_map[transition.node_id()].unwrap());
                    bad.set_nodeid(nodes_map[bad.node_id()].unwrap());
                    equation.set_nodeid(nodes_map[equation.node_id()].unwrap());
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
            }
            frontier = self.migrate_logic(&latch_map, equation);
            let reach_new = self.new_or_node(reach, frontier);
            if reach != reach_new {
                reach = reach_new
            } else {
                dbg!(deep);
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
        aig.fraig();
        dbg!(aig.symbolic_mc_back());
    }

    #[test]
    fn test2() {
        let mut aig = Aig::from_file("./aigs/counter-3bit.aag").unwrap();
        println!("{}", aig);
        aig.fraig();
        dbg!(aig.symbolic_mc_back());
    }

    #[test]
    fn test3() {
        let mut aig = Aig::from_file(
            "/root/MC-Benchmark/hwmcc20/aig/2019/goel/crafted/paper_v3/paper_v3.aag",
        )
        .unwrap();
        println!("{}", aig);
        aig.fraig();
        println!("{}", aig);
        dbg!(aig.symbolic_mc());
    }
}
