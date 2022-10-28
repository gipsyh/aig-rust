use super::SatSolver;
use crate::{AigEdge, AigNodeId};
use minisat::Bool;

#[derive(Debug)]
pub struct Solver {
    solver: minisat::Solver,
    vars: Vec<Bool>,
    ret: Vec<AigEdge>,
}

impl SatSolver for Solver {
    fn add_input_node(&mut self, _node: AigNodeId) {
        self.vars.push(self.solver.new_lit());
    }

    fn add_and_node(&mut self, _node: AigNodeId, fanin0: AigEdge, fanin1: AigEdge) {
        let node = self.solver.new_lit();
        let fanin0 = self.edge_to_lit(fanin0);
        let fanin1 = self.edge_to_lit(fanin1);
        self.solver.add_clause([!fanin0, !fanin1, node]);
        self.solver.add_clause([fanin0, !node]);
        self.solver.add_clause([fanin1, !node]);
        self.vars.push(node);
    }

    fn new_round(&mut self) {}

    fn mark_cone(&mut self, _cones: &[AigEdge]) {}

    fn solve_without_mark_cone(&mut self, assumptions: &[AigEdge]) -> Option<&[AigEdge]> {
        let lits: Vec<Bool> = assumptions.iter().map(|e| self.edge_to_lit(*e)).collect();
        match self.solver.solve_under_assumptions(lits) {
            Ok(m) => {
                for i in 1..self.vars.len() {
                    self.ret.push(AigEdge::new(i, !m.value(&self.vars[i])))
                }
                Some(&self.ret)
            }
            Err(()) => None,
        }
    }
}

impl Solver {
    fn edge_to_lit(&self, e: AigEdge) -> Bool {
        if e.compl() {
            !self.vars[e.node_id()]
        } else {
            self.vars[e.node_id()]
        }
    }
}
