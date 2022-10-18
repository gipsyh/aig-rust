use crate::{Aig, AigNodeId, AigNodeType};
use std::{assert_matches::assert_matches, collections::VecDeque, vec};

impl Aig {
    fn eliminate_input(&mut self, eid: AigNodeId) {
        assert_matches!(self.nodes[eid].typ, AigNodeType::PrimeInput);
        let seen = vec![false; self.num_nodes()];
        let mut inqueue = vec![false; self.num_nodes()];
        let mut queue = VecDeque::new();
        let mut value = vec![None; self.num_nodes()];
        value[eid] = Some(true);
        for out in &self.nodes[eid].fanouts {
            queue.push_back(out.node_id());
            inqueue[out.node_id()] = true;
        }
        while !queue.is_empty() {
            let now = queue.pop_front().unwrap();
            inqueue[now] = false;
            let fanin0 = self.nodes[now].fanin0();
            let fanin1 = self.nodes[now].fanin1();
            if inqueue[fanin0.node_id()] || inqueue[fanin1.node_id()] {
                continue;
            }
        }
    }
}
