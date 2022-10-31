use crate::Aig;
use std::{assert_matches::assert_matches, mem::swap};

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
        for i in &self.cinputs {
            if let crate::AigNodeType::PrimeInput = self.nodes[*i].typ {
                eliminate.push(*i);
            }
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
        let mut inputs = self.cinputs.clone();
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
            if self.num_nodes() > 5000 {
                let nodes_map = self.cleanup_redundant(&[frontier, reach, transition]);
                reach.set_nodeid(nodes_map[reach.node_id()].unwrap());
                frontier.set_nodeid(nodes_map[frontier.node_id()].unwrap());
                transition.set_nodeid(nodes_map[transition.node_id()].unwrap());
                bad.set_nodeid(nodes_map[bad.node_id()].unwrap());
                for input in &mut inputs {
                    *input = nodes_map[*input].unwrap();
                }
                for (x, y) in &mut latch_map {
                    *x = nodes_map[*x].unwrap();
                    *y = nodes_map[*y].unwrap();
                }
                println!("after cleanup: {}", self.num_nodes());
            }
            if self.sat_solver.solve(&[bad, frontier]).is_some() {
                return false;
            }
            let mut equation = self.new_and_node(frontier, transition);
            for iid in &inputs {
                assert_matches!(self.nodes[*iid].typ, crate::AigNodeType::PrimeInput);
                equation = self.eliminate_input(*iid, vec![equation])[0];
                // dbg!(self.num_nodes());
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
