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

    fn node_cnf(&self, node: AigNodeId) -> Vec<Vec<i32>> {
        let mut ret = Vec::new();
        let mut flag = vec![false; self.num_nodes()];
        flag[node] = true;
        for id in (0..self.num_nodes()).rev() {
            if flag[id] && self.nodes[id].is_and() {
                flag[self.nodes[id].fanin0().node_id()] = true;
                flag[self.nodes[id].fanin1().node_id()] = true;
                let cnf = self.and_node_cnf(id);
                for clause in cnf {
                    ret.push(clause)
                }
            }
        }
        ret
    }

    fn cnf(&self, logic: AigEdge) -> Vec<Vec<i32>> {
        let mut ret = self.node_cnf(logic.node_id());
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

    pub fn equivalence_check(&self, x: AigEdge, y: AigEdge) -> bool {
        let mut cnf = self.node_cnf(x.node_id());
        cnf.append(&mut self.node_cnf(y.node_id()));
        let x = self.edge_to_i32(x);
        let y = self.edge_to_i32(y);
        cnf.push(vec![x, y]);
        cnf.push(vec![-x, -y]);
        let sat = Certificate::try_from(cnf).unwrap();
        !matches!(sat, Certificate::SAT(_))
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
