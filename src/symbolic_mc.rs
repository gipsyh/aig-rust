use crate::Aig;
use std::assert_matches::assert_matches;

impl Aig {
    pub fn symbolic_mc(&mut self) -> bool {
        if self.bads.is_empty() {
            return true;
        }
        let mut reach = self.latch_init_equation();
        let mut inputs = self.cinputs.clone();
        // inputs.reverse();
        let (latch_map, transition) = self.transfer_latch_outputs_into_pinputs();
        let mut bad = self.bads[0];
        let bads = self.bads.clone();
        for b in &bads[1..] {
            bad = self.new_or_node(bad, *b);
        }
        let mut deep = 0;
        loop {
            deep += 1;
            dbg!(deep, self.num_nodes());
            if self
                .sat_solver
                .solve_under_assumptions([bad, reach])
                .is_some()
            {
                return false;
            }
            let mut equation = self.new_and_node(reach, transition);
            for iid in &inputs {
                assert_matches!(self.nodes[*iid].typ, crate::AigNodeType::PrimeInput);
                equation = self.eliminate_input(*iid, vec![equation])[0];
            }
            equation = self.migrate_logic(&latch_map, equation);
            let reach_new = self.new_or_node(reach, equation);
            if self
                .sat_solver
                .solve_under_assumptions([reach_new, !reach])
                .is_some()
            {
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
    use crate::Aig;
    #[test]
    fn test1() {
        let mut aig =
            Aig::from_file("/root/MC-Benchmark/examples/counter/10bit/counter.aag").unwrap();
        println!("{}", aig);
        aig.fraig();
        dbg!(aig.symbolic_mc());
    }

    #[test]
    fn test2() {
        let mut aig = Aig::from_file("./aigs/counter-3bit.aag").unwrap();
        println!("{}", aig);
        aig.fraig();
        dbg!(aig.symbolic_mc());
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
