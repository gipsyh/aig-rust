use crate::{Aig, AigNodeId, AigNodeType};

impl Aig {
    fn eliminate(&mut self, eid: AigNodeId) {
        if let AigNodeType::PrimeInput = self.nodes[eid].typ {
            
        } else {
            panic!();
        }
    }
}
