use std::assert_matches::assert_matches;

use crate::Aig;

impl Aig {
    pub fn symbolic_mc(&mut self) -> bool {
        let mut reach = self.latch_init_equation();
        let inputs = self.cinputs.clone();
        let (latch_map, transition) = self.transfer_latch_outputs_into_pinputs();
        for _ in 0..2 {
            let mut equation = self.new_and_node(reach, transition);
            for iid in &inputs {
                assert_matches!(self.nodes[*iid].typ, crate::AigNodeType::PrimeInput);
                equation = self.eliminate_input(*iid, vec![equation])[0];
            }
            equation = self.migrate_logic(&latch_map, equation);
            reach = self.new_or_node(reach, equation);
            println!("{}", self);
            dbg!(&reach);
        }
        true
    }
}
