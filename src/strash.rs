use crate::{AigEdge, AigNode, AigNodeId};
use std::collections::HashMap;

#[derive(Debug)]
pub struct Strash {
    map: HashMap<(AigEdge, AigEdge), AigNodeId>,
}

impl Strash {
    pub fn _find(&self, fanin0: AigEdge, fanin1: AigEdge) -> Option<AigEdge> {
        assert!(fanin0 < fanin1);
        self.map
            .get(&(fanin0, fanin1))
            .map(|r| AigEdge::new(*r, false))
    }

    pub fn _add(&mut self, fanin0: AigEdge, fanin1: AigEdge, node: AigNodeId) {
        assert!(fanin0 < fanin1);
        self.map.insert((fanin0, fanin1), node);
    }

    pub fn _remove(&mut self, fanin0: AigEdge, fanin1: AigEdge) {
        assert!(fanin0 < fanin1);
        assert!(self.map.remove(&(fanin0, fanin1)).is_some());
    }

    pub fn _new(nodes: &[AigNode]) -> Self {
        let mut map = HashMap::new();
        for node in nodes.iter() {
            if node.is_and() {
                let fanin0 = node.fanin0();
                let fanin1 = node.fanin1();
                assert!(fanin0.node_id() < fanin1.node_id());
                match map.get(&(fanin0, fanin1)) {
                    Some(_) => {
                        todo!()
                    }
                    None => {
                        map.insert((fanin0, fanin1), node.id);
                    }
                }
            }
        }
        Self { map }
    }
}
