use super::{Clause, SatSolver, CNF};
use crate::{AigEdge, AigNodeId};
use minisat::Lit;

#[derive(Debug)]
pub struct Solver {
    solver: minisat::Solver,
    vars: Vec<Lit>,
    ret: Vec<AigEdge>,
}

impl SatSolver for Solver {
    fn add_input_node(&mut self, _node: AigNodeId) {
        self.vars.push(self.solver.new_lit());
    }

    fn add_and_node(&mut self, node: AigNodeId, fanin0: AigEdge, fanin1: AigEdge) {
        assert!(fanin0.node_id() < fanin1.node_id());
        assert!(fanin1.node_id() < node);
        assert!(fanin0.node_id() != 0 && fanin1.node_id() != 0);
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
        if assumptions
            .iter()
            .any(|e| *e == AigEdge::constant_edge(false))
        {
            return None;
        }
        self.ret.clear();
        let lits: Vec<Lit> = assumptions.iter().map(|e| self.edge_to_lit(*e)).collect();
        match self.solver.solve_under_assumptions(lits) {
            Ok(m) => {
                for i in 1..self.vars.len() {
                    self.ret.push(AigEdge::new(i, !m.lit_value(&self.vars[i])))
                }
                Some(&self.ret)
            }
            Err(_) => None,
        }
    }
}

impl Solver {
    fn edge_to_lit(&self, e: AigEdge) -> Lit {
        if e.compl() {
            !self.vars[e.node_id()]
        } else {
            self.vars[e.node_id()]
        }
    }
}

impl Solver {
    #[allow(dead_code)]
    pub fn new() -> Self {
        let mut solver = minisat::Solver::new();
        let vars = vec![solver.new_lit()];
        Self {
            solver,
            vars,
            ret: Vec::new(),
        }
    }

    pub fn add_cnf(&mut self, cnf: &CNF) {
        for clause in cnf.iter() {
            self.add_clause(clause)
        }
    }

    pub fn add_clause(&mut self, clause: &Clause) {
        if clause.iter().any(|e| *e == AigEdge::constant_edge(false)) {
            panic!();
        }
        let clause: Vec<Lit> = clause.iter().map(|e| self.edge_to_lit(*e)).collect();
        self.solver.add_clause(clause)
    }
}
