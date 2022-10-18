use crate::{Aig, AigNodeId, AigNodeType};
use std::{assert_matches::assert_matches, collections::VecDeque, vec};

impl Aig {
    fn eliminate_input(&mut self, eid: AigNodeId) {
        assert_matches!(self.nodes[eid].typ, AigNodeType::PrimeInput);
        let flag = vec![false; self.num_nodes()];
        // let mut queue = VecDeque::new();
        // let mut value = vec![None; self.num_nodes()];
        // value[eid] = Some(true);
        // // queue.push_back(eid)
        // while !queue.is_empty() {

        // }
    }
}
