use crate::{Aig, AigEdge, AigNode, AigNodeId};
use minisat::{Bool, Solver};

#[derive(Debug)]
pub struct SatSolver {
    solver: Solver,
    vars: Vec<Bool>,
}

impl SatSolver {
    fn node_to_lit(&self, n: &AigNode) -> Bool {
        self.vars[n.node_id()]
    }

    fn edge_to_lit(&self, e: AigEdge) -> Bool {
        if e.compl() {
            !self.vars[e.node_id()]
        } else {
            self.vars[e.node_id()]
        }
    }
}

impl SatSolver {
    pub fn solve_under_assumptions<I: IntoIterator<Item = AigEdge>>(
        &mut self,
        edges: I,
    ) -> Option<Vec<bool>> {
        let lits: Vec<Bool> = edges.into_iter().map(|e| self.edge_to_lit(e)).collect();
        match self.solver.solve_under_assumptions(lits) {
            Ok(m) => {
                let mut ret = Vec::new();
                for var in &self.vars[1..] {
                    ret.push(m.value(var))
                }
                Some(ret)
            }
            Err(()) => None,
        }
    }

    pub fn equivalence_check(&mut self, x: AigEdge, y: AigEdge) -> Option<Vec<bool>> {
        match self.solve_under_assumptions([x, !y]) {
            Some(ret) => Some(ret),
            None => self.solve_under_assumptions([!x, y]),
        }
    }

    pub fn equivalence_check_xy_z(
        &mut self,
        x: AigEdge,
        y: AigEdge,
        z: AigEdge,
    ) -> Option<Vec<bool>> {
        if let Some(ret) = self.solve_under_assumptions([x, y, !z]) {
            return Some(ret);
        }
        if let Some(ret) = self.solve_under_assumptions([!x, z]) {
            return Some(ret);
        }
        self.solve_under_assumptions([!y, z])
    }
}

impl SatSolver {
    pub fn new_input_node(&mut self) {
        self.vars.push(self.solver.new_lit());
    }

    pub fn new_node(&mut self, fanin0: AigEdge, fanin1: AigEdge) {
        let node = self.solver.new_lit();
        let fanin0 = self.edge_to_lit(fanin0);
        let fanin1 = self.edge_to_lit(fanin1);
        self.solver.add_clause([!fanin0, !fanin1, node]);
        self.solver.add_clause([fanin0, !node]);
        self.solver.add_clause([fanin1, !node]);
        self.vars.push(node);
    }
}

impl Default for SatSolver {
    fn default() -> Self {
        Self {
            solver: Solver::new(),
            vars: Default::default(),
        }
    }
}

impl Aig {
    fn add_node_clause(&mut self, node: AigNodeId) {
        assert!(self.nodes[node].is_and());
        let fanin0 = self.sat_solver.edge_to_lit(self.nodes[node].fanin0());
        let fanin1 = self.sat_solver.edge_to_lit(self.nodes[node].fanin1());
        let node = self.sat_solver.node_to_lit(&self.nodes[node]);
        self.sat_solver.solver.add_clause([!fanin0, !fanin1, node]);
        self.sat_solver.solver.add_clause([fanin0, !node]);
        self.sat_solver.solver.add_clause([fanin1, !node]);
    }

    pub fn setup_sat_solver(&mut self) {
        self.sat_solver.vars.push(Bool::Const(true));
        for i in self.nodes_range() {
            self.sat_solver.vars.push(self.sat_solver.solver.new_lit());
            if self.nodes[i].is_and() {
                self.add_node_clause(i);
            }
        }
    }
}

impl Aig {
    // fn node_cnf(&self, node: AigNodeId) -> Vec<Vec<i32>> {
    //     let mut ret = Vec::new();
    //     let mut flag = vec![false; self.num_nodes()];
    //     flag[node] = true;
    //     for id in self.nodes_range().rev() {
    //         if flag[id] && self.nodes[id].is_and() {
    //             flag[self.nodes[id].fanin0().node_id()] = true;
    //             flag[self.nodes[id].fanin1().node_id()] = true;
    //             let cnf = self.and_node_cnf(id);
    //             for clause in cnf {
    //                 ret.push(clause)
    //             }
    //         }
    //     }
    //     ret
    // }
    // fn cnf_sat(&self, cnf: Vec<Vec<i32>>) -> Option<Vec<(AigNodeId, bool)>> {
    //     let solver = Solver::new();

    //     solver.add_clause(lits)
    //     let ret = match Certificate::try_from(cnf).unwrap() {
    //         Certificate::SAT(counter) => {
    //             let mut ret = Vec::new();
    //             for x in counter {
    //                 ret.push((x.abs() as usize, x.is_positive()))
    //             }
    //             Some(ret)
    //         }
    //         Certificate::UNSAT => None,
    //     };
    //     ret
    // }
}

#[cfg(test)]
mod tests {
    use crate::{Aig, AigEdge};

    #[test]
    fn test_cec1() {
        let mut aig = Aig::from_file("aigs/cec1.aag").unwrap();
        assert!(aig
            .sat_solver
            .equivalence_check(aig.outputs[0], aig.outputs[1])
            .is_none());
    }

    #[test]
    fn test_cec2() {
        let mut aig = Aig::from_file("aigs/cec2.aag").unwrap();
        dbg!(aig
            .sat_solver
            .equivalence_check(aig.outputs[0], aig.outputs[1]));
        assert!(aig
            .sat_solver
            .equivalence_check(aig.outputs[0], aig.outputs[1])
            .is_none());
    }

    #[test]
    fn test_cec_xy_z() {
        let mut aig = Aig::from_file("aigs/cec1.aag").unwrap();
        assert!(aig
            .sat_solver
            .equivalence_check_xy_z(AigEdge::new(4, true), 2.into(), 2.into())
            .is_none());
    }
}
