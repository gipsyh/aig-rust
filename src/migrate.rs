use std::vec;

use crate::{Aig, AigEdge, AigNodeId};

impl Aig {
    pub fn migrate_logic(
        &mut self,
        nodes: &Vec<(AigNodeId, AigNodeId)>,
        logic: AigEdge,
    ) -> AigEdge {
        let flag = self.logic_cone(logic);
        let mut map = vec![None; self.num_nodes()];
        for (src, dest) in nodes {
            map[*src] = Some(AigEdge::new(*dest, false));
        }
        for id in 0..self.num_nodes() {
            if flag[id] && map[id].is_none() {
                map[id] = Some(if self.nodes[id].is_and() {
                    let fanin0 = self.nodes[id].fanin0();
                    let fanin1 = self.nodes[id].fanin1();
                    let mut new_fanin0 = map[fanin0.node_id()].unwrap();
                    let mut new_fanin1 = map[fanin1.node_id()].unwrap();
                    if fanin0.compl() {
                        new_fanin0 = !new_fanin0;
                    }
                    if fanin1.compl() {
                        new_fanin1 = !new_fanin1;
                    }
                    self.new_and_node(new_fanin0, new_fanin1)
                } else {
                    AigEdge::new(id, false)
                })
            }
        }
        if logic.compl() {
            !map[logic.node_id()].unwrap()
        } else {
            map[logic.node_id()].unwrap()
        }
    }
}
