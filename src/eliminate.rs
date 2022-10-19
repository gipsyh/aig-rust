use crate::{Aig, AigEdge, AigNodeId, AigNodeType};
use std::{assert_matches::assert_matches, vec};

impl Aig {
    fn eliminate_input_polarity(
        &mut self,
        eid: AigNodeId,
        polarity: bool,
        ignores_begin: AigNodeId,
    ) -> Vec<AigEdge> {
        assert_matches!(self.nodes[eid].typ, AigNodeType::PrimeInput);
        assert!(eid < ignores_begin);
        let mut flag = vec![false; self.num_nodes()];
        let mut value = vec![None; self.num_nodes()];
        flag[eid] = true;
        for node in 0..self.num_nodes() {
            if !flag[node] {
                continue;
            }
            for out in &self.nodes[node].fanouts {
                if out.node_id() < ignores_begin {
                    flag[out.node_id()] = true;
                }
            }
            if node == eid {
                value[eid] = Some(AigEdge::new(0, !polarity));
                continue;
            }
            assert!(self.nodes[node].is_and());
            let mut fanin0 = self.nodes[node].fanin0();
            let mut fanin1 = self.nodes[node].fanin1();
            assert!(value[fanin0.node_id()].is_some() || value[fanin1.node_id()].is_some());
            if let Some(edge) = value[fanin0.node_id()] {
                if fanin0.compl() {
                    fanin0 = !edge;
                } else {
                    fanin0 = edge;
                }
            }
            if let Some(edge) = value[fanin1.node_id()] {
                if fanin1.compl() {
                    fanin1 = !edge;
                } else {
                    fanin1 = edge;
                }
            }
            value[node] = Some(if fanin0.node_id() == 0 {
                if fanin0.compl() {
                    AigEdge::new(0, true)
                } else {
                    fanin1
                }
            } else if fanin1.node_id() == 0 {
                if fanin1.compl() {
                    AigEdge::new(0, true)
                } else {
                    fanin0
                }
            } else {
                self.new_and_node(fanin0, fanin1)
            });
        }
        let mut ret = Vec::new();
        for output in &self.outputs {
            ret.push(match value[output.node_id()] {
                Some(edge) => {
                    if output.compl() {
                        !edge
                    } else {
                        edge
                    }
                }
                None => *output,
            });
        }
        ret
    }

    pub fn eliminate_input(&mut self, eid: AigNodeId) {
        let num_nodes = self.num_nodes();
        let out_true = self.eliminate_input_polarity(eid, true, num_nodes);
        dbg!(out_true);
        let out_false = self.eliminate_input_polarity(eid, false, num_nodes);
        dbg!(out_false);
    }
}
