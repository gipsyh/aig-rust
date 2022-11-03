use super::SatSolver;
use crate::{AigEdge, AigNodeId};
use abc_glucose::{Lit, Var};

#[derive(Debug)]
pub struct Solver {
    solver: abc_glucose::Solver,
    cex: Vec<AigEdge>,
}

impl Solver {
    pub fn new() -> Self {
        let mut solver = abc_glucose::Solver::new();
        assert_eq!(solver.add_var(), abc_glucose::Var::from(0));
        Self {
            solver,
            cex: Vec::new(),
        }
    }
}

impl Solver {
    fn node_to_var(node: AigNodeId) -> Var {
        Var::from(node as i32)
    }

    fn edge_to_lit(edge: AigEdge) -> Lit {
        Lit::new(Self::node_to_var(edge.node_id()), edge.compl())
    }
}

impl SatSolver for Solver {
    fn add_input_node(&mut self, node: AigNodeId) {
        let var = self.solver.add_var();
        assert_eq!(var, Self::node_to_var(node));
    }

    fn add_and_node(&mut self, node: AigNodeId, fanin0: AigEdge, fanin1: AigEdge) {
        assert!(fanin0.node_id() < fanin1.node_id());
        assert!(fanin1.node_id() < node);
        let node = Self::node_to_var(node);
        let var = self.solver.add_var();
        assert_eq!(var, node);
        assert_eq!(var, node);
        self.solver
            .set_fanin(node, Self::edge_to_lit(fanin0), Self::edge_to_lit(fanin1));
    }

    fn new_round(&mut self) {
        self.solver.new_round()
    }

    fn mark_cone(&mut self, cones: &[AigEdge]) {
        for c in cones {
            self.solver.mark_cone(Self::node_to_var(c.node_id()))
        }
    }

    fn solve_without_mark_cone(&mut self, assumptions: &[AigEdge]) -> Option<&[AigEdge]> {
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
}

#[cfg(test)]
mod tests {
    use crate::{sat::SatSolver, AigEdge};

    use super::Solver;

    #[test]
    fn test() {
        let mut solver = Solver::new();
        solver.add_input_node(1);
        solver.add_input_node(2);
        solver.add_and_node(3, 1.into(), 2.into());
        solver.new_round();
        solver.mark_cone(&[3.into()]);
        let ret = solver.solve_without_mark_cone(&[AigEdge::new(3, true)]);
        dbg!(ret);
    }
}
