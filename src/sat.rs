use splr::Certificate;

use crate::{Aig, AigEdge, AigNodeId};

impl Aig {
    fn edge_to_i32(&self, edge: AigEdge) -> i32 {
        if edge.compl() {
            -(edge.node_id() as i32)
        } else {
            edge.node_id() as i32
        }
    }

    fn and_node_cnf(&self, node: AigNodeId) -> Vec<Vec<i32>> {
        assert!(self.nodes[node].is_and());
        let fanin0 = self.edge_to_i32(self.nodes[node].fanin0());
        let fanin1 = self.edge_to_i32(self.nodes[node].fanin1());
        let nodeid = node as i32;
        vec![
            vec![-fanin0, -fanin1, nodeid],
            vec![fanin0, -nodeid],
            vec![fanin1, -nodeid],
        ]
    }

    fn cnf(&self, logic: AigEdge) -> Vec<Vec<i32>> {
        let mut ret = Vec::new();
        let mut flag = vec![false; self.num_nodes()];
        flag[logic.node_id()] = true;
        for id in (0..self.num_nodes()).rev() {
            if flag[id] {
                if self.nodes[id].is_and() {
                    flag[self.nodes[id].fanin0().node_id()] = true;
                    flag[self.nodes[id].fanin1().node_id()] = true;
                    let cnf = self.and_node_cnf(id);
                    for clause in cnf {
                        ret.push(clause)
                    }
                }
            }
        }
        ret.push(vec![self.edge_to_i32(logic)]);
        ret
    }

    pub fn sat(&self, logic: AigEdge) -> bool {
        println!("begin sat");
        let cnf = self.cnf(logic);
        let sat = Certificate::try_from(cnf).unwrap();
        println!("end sat");
        matches!(sat, Certificate::SAT(_))
    }
}

#[cfg(test)]
mod tests {
    use crate::Aig;
    use splr::Certificate;

    #[test]
    fn test() {
        let aig = Aig::from_file("aigs/xor.aag").unwrap();
        let logic = aig.outputs[0];
        let cnf = aig.cnf(logic);
        println!("{}", aig);
        println!("{:?}", cnf);
        let sat = Certificate::try_from(cnf).unwrap();
        dbg!(sat);
    }
}
