use crate::{Aig, AigNodeId, AigNodeType};
use std::assert_matches::assert_matches;

impl Aig {
    fn eliminate_input(&mut self, eid: AigNodeId) {
        assert_matches!(self.nodes[eid].typ, AigNodeType::PrimeInput);
    }
}
