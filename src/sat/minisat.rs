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
        self.ret.clear();
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

    fn edge_to_lit_with_dual(&self, e: AigEdge) -> (Bool, Bool) {
        if e.node_id() == 0 {
            if e.compl() {
                (!self.vars[0], self.vars[0])
            } else {
                (self.vars[0], !self.vars[0])
            }
        } else {
            if e.compl() {
                (self.vars[e.node_id() * 2], self.vars[e.node_id() * 2 - 1])
            } else {
                (self.vars[e.node_id() * 2 - 1], self.vars[e.node_id() * 2])
            }
        }
    }
}

impl Solver {
    #[allow(dead_code)]
    pub fn new() -> Self {
        let solver = minisat::Solver::new();
        let vars = vec![Bool::Const(false)];
        Self {
            solver,
            vars,
            ret: Vec::new(),
        }
    }

    pub fn add_clause(&mut self, clause: &[AigEdge]) {
        let clause: Vec<Bool> = clause.iter().map(|e| self.edge_to_lit(*e)).collect();
        self.solver.add_clause(clause)
    }

    pub fn add_input_node_with_dual(&mut self, _node: AigNodeId) -> (AigNodeId, AigNodeId) {
        self.vars.push(self.solver.new_lit());
        self.vars.push(self.solver.new_lit());
        (self.vars.len() - 2, self.vars.len() - 1)
    }

    pub fn add_and_node_with_dual(
        &mut self,
        _node: AigNodeId,
        fanin0: AigEdge,
        fanin1: AigEdge,
    ) -> (AigNodeId, AigNodeId) {
        let node_pos = self.solver.new_lit();
        let node_neg = self.solver.new_lit();
        let (fanin0_pos, fanin0_neg) = self.edge_to_lit_with_dual(fanin0);
        let (fanin1_pos, fanin1_neg) = self.edge_to_lit_with_dual(fanin1);
        self.solver.add_clause([!fanin0_pos, !fanin1_pos, node_pos]);
        self.solver.add_clause([fanin0_pos, !node_pos]);
        self.solver.add_clause([fanin1_pos, !node_pos]);
        self.solver.add_clause([fanin0_neg, fanin1_neg, !node_neg]);
        self.solver.add_clause([!fanin0_neg, node_neg]);
        self.solver.add_clause([!fanin1_neg, node_neg]);
        self.vars.push(node_pos);
        self.vars.push(node_neg);
        (self.vars.len() - 2, self.vars.len() - 1)
    }

    pub fn add_equal_node_with_dual(&mut self, left: AigEdge, right: AigEdge) {
        let (left_pos, left_neg) = self.edge_to_lit_with_dual(left);
        let (right_pos, right_neg) = self.edge_to_lit_with_dual(right);
        let mut closure = |x: Bool, y: Bool| {
            self.solver.add_clause([x, !y]);
            self.solver.add_clause([!x, y]);
        };
        closure(left_pos, right_pos);
        closure(left_neg, right_neg);
    }

    pub fn new_variable(&mut self) -> usize {
        self.vars.push(self.solver.new_lit());
        self.vars.len() - 1
    }

    pub fn solve_with_dual(
        &mut self,
        assumptions: &[AigEdge],
        dual_assumptions: &[AigEdge],
    ) -> Option<&[AigEdge]> {
        self.ret.clear();
        let mut lits: Vec<Bool> = assumptions
            .iter()
            .map(|e| self.edge_to_lit_with_dual(*e).0)
            .collect();
        // let dual_lits: Vec<Bool> = dual_assumptions
        //     .iter()
        //     .map(|e| self.edge_to_lit(*e))
        //     .collect();
        // lits.extend(dual_lits);
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
