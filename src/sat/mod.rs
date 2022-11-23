pub mod abc_glucose;
pub mod minisat;

use crate::{Aig, AigEdge, AigNodeId};
use std::{
    fmt::Debug,
    ptr::{from_raw_parts_mut, metadata},
};

pub trait SatSolver: Debug {
    fn add_input_node(&mut self, node: AigNodeId);

    fn add_and_node(&mut self, node: AigNodeId, fanin0: AigEdge, fanin1: AigEdge);

    fn new_round(&mut self);

    fn mark_cone(&mut self, cones: &[AigEdge]);

    fn solve_without_mark_cone(&mut self, assumptions: &[AigEdge]) -> Option<&[AigEdge]>;

    fn solve(&mut self, assumptions: &[AigEdge]) -> Option<&[AigEdge]> {
        self.new_round();
        self.mark_cone(assumptions);
        self.solve_without_mark_cone(assumptions)
    }

    fn equivalence_check(&mut self, x: AigEdge, y: AigEdge) -> Option<&[AigEdge]> {
        self.new_round();
        self.mark_cone(&[x, y]);
        let m = metadata(self as *const Self);
        let fake: *mut Self = from_raw_parts_mut(self as *mut Self as *mut (), m);
        if let Some(ret) = self.solve_without_mark_cone(&[x, !y]) {
            return Some(ret);
        }
        unsafe { fake.as_mut().unwrap() }.solve_without_mark_cone(&[!x, y])
    }

    fn equivalence_check_xy_z(&mut self, x: AigEdge, y: AigEdge, z: AigEdge) -> Option<&[AigEdge]> {
        self.new_round();
        self.mark_cone(&[x, y, z]);
        let m = metadata(self as *const Self);
        let fake: *mut Self = from_raw_parts_mut(self as *mut Self as *mut (), m);
        if let Some(ret) = self.solve_without_mark_cone(&[x, y, !z]) {
            return Some(ret);
        }
        if let Some(ret) = unsafe { fake.as_mut().unwrap() }.solve_without_mark_cone(&[!x, z]) {
            return Some(ret);
        }
        unsafe { fake.as_mut().unwrap() }.solve_without_mark_cone(&[!y, z])
    }
}

impl Aig {
    pub fn setup_sat_solver(&mut self) {
        for i in self.nodes_range() {
            if self.nodes[i].is_and() {
                self.sat_solver
                    .add_and_node(i, self.nodes[i].fanin0(), self.nodes[i].fanin1())
            } else {
                assert!(self.nodes[i].is_cinput());
                self.sat_solver.add_input_node(i)
            }
        }
    }
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
