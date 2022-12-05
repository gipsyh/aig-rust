use glucose::{Lit, SimpSolver, Var};

use crate::{AigEdge, AigNodeId};

#[derive(Debug)]
pub struct Solver {
    solver: SimpSolver,
    cex: Vec<AigEdge>,
}

impl Solver {
    fn node_to_var(node: AigNodeId) -> Var {
        Var::from(node as i32)
    }

    fn edge_to_lit(edge: AigEdge) -> Lit {
        Lit::new(Self::node_to_var(edge.node_id()), edge.compl())
    }
}

impl Solver {
    pub fn new() -> Self {
        let mut solver = glucose::SimpSolver::new();
        assert_eq!(solver.new_var(), glucose::Var::from(0));
        Self {
            solver,
            cex: Vec::new(),
        }
    }

    pub fn add_and_node(&mut self, _node: AigNodeId, fanin0: AigEdge, fanin1: AigEdge) {
        let node = self.solver.new_var().into();
        let fanin0 = Self::edge_to_lit(fanin0);
        let fanin1 = Self::edge_to_lit(fanin1);
        self.solver.add_clause(&[!fanin0, !fanin1, node]);
        self.solver.add_clause(&[fanin0, !node]);
        self.solver.add_clause(&[fanin1, !node]);
    }

    pub fn add_input_node(&mut self, node: AigNodeId) {
        let var = self.solver.new_var();
        assert_eq!(var, Self::node_to_var(node));
    }

    pub fn solve(&mut self, assumptions: &[AigEdge]) -> Option<&[AigEdge]> {
        if assumptions
            .iter()
            .any(|e| *e == AigEdge::constant_edge(false))
        {
            return None;
        }
        let assumptions: Vec<Lit> = assumptions
            .iter()
            .map(|e| Lit::new(Self::node_to_var(e.node_id()), e.compl()))
            .collect();

        match self.solver.solve(&assumptions) {
            Some(cex) => {
                self.cex = cex
                    .iter()
                    .chain(assumptions.iter())
                    .map(|l| AigEdge::new((Into::<i32>::into(l.var())) as usize, l.compl()))
                    .filter(|e| e.node_id() > 0)
                    .collect();
                Some(&self.cex)
            }
            None => None,
        }
    }

    pub fn add_clause(&mut self, clause: &[AigEdge]) {
        let clause: Vec<Lit> = clause
            .iter()
            .map(|e| Lit::new(Self::node_to_var(e.node_id()), e.compl()))
            .collect();
        self.solver.add_clause(&clause);
    }
}
