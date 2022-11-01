use crate::{Aig, AigEdge, AigNodeId};
use std::vec;

impl Aig {
    #[inline]
    fn eliminate_input_polarity(
        &mut self,
        eid: AigNodeId,
        polarity: bool,
        ignores_begin: AigNodeId,
        observe_cone: &[bool],
        observes: Vec<AigEdge>,
    ) -> (Vec<AigEdge>, Vec<AigEdge>) {
        debug_assert!(self.nodes[eid].is_prime_input());
        debug_assert!(eid < ignores_begin);
        debug_assert!(ignores_begin == observe_cone.len());
        let mut flag = vec![false; ignores_begin];
        let mut value = vec![None; ignores_begin];
        flag[eid] = true;
        for node in 0..ignores_begin {
            if !flag[node] || !observe_cone[node] {
                continue;
            }
            for out in &self.nodes[node].fanouts {
                if out.node_id() < ignores_begin {
                    flag[out.node_id()] = true;
                }
            }
            if node == eid {
                value[eid] = Some(Aig::constant_edge(!polarity));
                continue;
            }
            debug_assert!(self.nodes[node].is_and());
            let mut fanin0 = self.nodes[node].fanin0();
            let mut fanin1 = self.nodes[node].fanin1();
            debug_assert!(value[fanin0.node_id()].is_some() || value[fanin1.node_id()].is_some());
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
            value[node] = Some(self.new_and_node(fanin0, fanin1));
        }
        let mut ret_out = Vec::new();
        let mut ret_ob = Vec::new();
        for output in &self.outputs {
            ret_out.push(match value[output.node_id()] {
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
        for observe in &observes {
            ret_ob.push(match value[observe.node_id()] {
                Some(edge) => {
                    if observe.compl() {
                        !edge
                    } else {
                        edge
                    }
                }
                None => *observe,
            });
        }
        (ret_out, ret_ob)
    }

    #[inline]
    pub fn eliminate_input(&mut self, eid: AigNodeId, observes: Vec<AigEdge>) -> Vec<AigEdge> {
        let fanin_cone = self.fanin_logic_cone(&observes);
        let num_nodes = self.num_nodes();
        let (out_true, ob_true) =
            self.eliminate_input_polarity(eid, true, num_nodes, &fanin_cone, observes.clone());
        let (out_false, ob_false) =
            self.eliminate_input_polarity(eid, false, num_nodes, &fanin_cone, observes);
        assert_eq!(out_true.len(), out_false.len());
        assert_eq!(ob_true.len(), ob_false.len());
        let mut out = Vec::new();
        let mut ob = Vec::new();
        for id in 0..out_true.len() {
            out.push(self.new_or_node(out_true[id], out_false[id]));
        }
        for id in 0..ob_true.len() {
            ob.push(self.new_or_node(ob_true[id], ob_false[id]));
        }
        self.outputs = out;
        ob
    }
}
