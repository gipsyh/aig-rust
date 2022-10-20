use crate::Aig;
use std::assert_matches::assert_matches;

impl Aig {
    pub fn symbolic_mc(&mut self) -> bool {
        if self.bads.is_empty() {
            return true;
        }
        let mut reach = self.latch_init_equation();
        let inputs = self.cinputs.clone();
        let (latch_map, transition) = self.transfer_latch_outputs_into_pinputs();
        let mut bad = self.bads[0];
        let bads = self.bads.clone();
        for b in &bads[1..] {
            bad = self.new_or_node(bad, *b);
        }
        let mut deep = 0;
        loop {
            deep += 1;
            dbg!(deep);
            dbg!(self.num_nodes());
            let bad_check = self.new_and_node(bad, reach);
            if self.sat(bad_check) {
                dbg!(deep);
                return false;
            }
            let mut equation = self.new_and_node(reach, transition);
            for iid in &inputs {
                assert_matches!(self.nodes[*iid].typ, crate::AigNodeType::PrimeInput);
                equation = self.eliminate_input(*iid, vec![equation])[0];
            }
            equation = self.migrate_logic(&latch_map, equation);
            let reach_new = self.new_or_node(reach, equation);
            let reach_increment_check = self.new_and_node(reach_new, !reach);
            if self.sat(reach_increment_check) {
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
    fn test_from_file() {
        let mut aig = Aig::from_file("aigs/counter-3bit.aag").unwrap();
        println!("{}", aig);
        dbg!(aig.symbolic_mc());
    }
}
