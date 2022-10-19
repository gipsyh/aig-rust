use crate::{Aig, AigEdge};

impl Aig {
    pub fn setup_strash(&mut self) {
        for and_id in 0..self.num_nodes() {
            if !self.nodes[and_id].is_and() {
                continue;
            }
            let fanin0 = self.nodes[and_id].fanin0();
            let fanin1 = self.nodes[and_id].fanin1();
            assert!(fanin0.node_id() < fanin1.node_id());
            match self.strash_map.get(&(fanin0, fanin1)) {
                Some(id) => {
                    todo!()
                }
                None => {
                    self.strash_map.insert((fanin0, fanin1), and_id);
                }
            }
        }
    }
}
