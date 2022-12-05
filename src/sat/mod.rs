pub mod abc_glucose;
pub mod glucose;
pub mod minisat;

use crate::{Aig, AigEdge, AigNodeId};
use std::{
    collections::HashMap,
    fmt::Debug,
    ops::{Add, Deref, DerefMut, Not},
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

#[derive(Clone, Debug)]
pub struct CNF {
    clauses: Vec<Clause>,
}

impl CNF {
    pub fn new() -> Self {
        Self {
            clauses: Vec::new(),
        }
    }

    pub fn add_clause(&mut self, clause: Clause) {
        self.clauses.push(clause);
    }

    pub fn value(&self, assignment: &[AigEdge]) -> bool {
        if self.clauses.is_empty() {
            return false;
        }
        let mut assigns = HashMap::new();
        for a in assignment {
            assert!(assigns.insert(a.node_id(), !a.compl()).is_none());
        }
        self.clauses.iter().all(|clause| {
            clause.lits.iter().any(|lit| {
                if let Some(v) = assigns.get(&lit.node_id()) {
                    *v != lit.compl()
                } else {
                    false
                }
            })
        })
    }
}

impl Deref for CNF {
    type Target = Vec<Clause>;

    fn deref(&self) -> &Self::Target {
        &self.clauses
    }
}

impl DerefMut for CNF {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.clauses
    }
}

impl Add for CNF {
    type Output = Self;

    fn add(mut self, mut rhs: Self) -> Self::Output {
        self.clauses.append(&mut rhs.clauses);
        self
    }
}

#[derive(Clone, Debug)]
pub struct Clause {
    lits: Vec<AigEdge>,
}

impl Clause {
    pub fn new() -> Self {
        Self { lits: Vec::new() }
    }
}

impl Deref for Clause {
    type Target = Vec<AigEdge>;

    fn deref(&self) -> &Self::Target {
        &self.lits
    }
}

impl DerefMut for Clause {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.lits
    }
}

impl From<&[AigEdge]> for Clause {
    fn from(value: &[AigEdge]) -> Self {
        Self {
            lits: value.to_vec(),
        }
    }
}

impl Not for Clause {
    type Output = Cube;

    fn not(self) -> Self::Output {
        let lits = self.lits.iter().map(|lit| !*lit).collect();
        Cube { lits }
    }
}

#[derive(Clone, Debug)]
pub struct DNF {
    cubes: Vec<Cube>,
}

impl DNF {
    pub fn new() -> Self {
        Self { cubes: Vec::new() }
    }

    pub fn add_cube(&mut self, cube: Cube) {
        self.cubes.push(cube);
    }

    pub fn value(&self, assignment: &[AigEdge]) -> bool {
        if self.cubes.is_empty() {
            return false;
        }
        let mut assigns = HashMap::new();
        for a in assignment {
            assert!(assigns.insert(a.node_id(), !a.compl()).is_none());
        }
        self.cubes.iter().any(|clause| {
            clause.lits.iter().all(|lit| {
                if let Some(v) = assigns.get(&lit.node_id()) {
                    *v != lit.compl()
                } else {
                    false
                }
            })
        })
    }
}

impl Deref for DNF {
    type Target = Vec<Cube>;

    fn deref(&self) -> &Self::Target {
        &self.cubes
    }
}

impl DerefMut for DNF {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cubes
    }
}

impl Add for DNF {
    type Output = Self;

    fn add(mut self, mut rhs: Self) -> Self::Output {
        self.cubes.append(&mut rhs.cubes);
        self
    }
}

impl Not for DNF {
    type Output = CNF;

    fn not(self) -> Self::Output {
        let mut cnf = CNF::new();
        for cube in self.cubes {
            cnf.add_clause(!cube);
        }
        cnf
    }
}

#[derive(Clone, Debug)]
pub struct Cube {
    lits: Vec<AigEdge>,
}

impl Cube {
    pub fn new() -> Self {
        Self { lits: Vec::new() }
    }
}

impl Deref for Cube {
    type Target = Vec<AigEdge>;

    fn deref(&self) -> &Self::Target {
        &self.lits
    }
}

impl DerefMut for Cube {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.lits
    }
}

impl From<&[AigEdge]> for Cube {
    fn from(value: &[AigEdge]) -> Self {
        Self {
            lits: value.to_vec(),
        }
    }
}

impl Not for Cube {
    type Output = Clause;

    fn not(self) -> Self::Output {
        let lits = self.lits.iter().map(|lit| !*lit).collect();
        Clause { lits }
    }
}

impl Aig {
    pub fn generate_cnf(&self) -> CNF {
        let mut cnf = CNF::new();
        for i in self.nodes_range() {
            if self.nodes[i].is_and() {
                let node: AigEdge = self.nodes[i].node_id().into();
                let fanin0 = self.nodes[i].fanin0();
                let fanin1 = self.nodes[i].fanin1();
                assert!(fanin0.node_id() > 0 && fanin1.node_id() > 0);
                cnf.add_clause(Clause::from([!fanin0, !fanin1, node].as_slice()));
                cnf.add_clause(Clause::from([fanin0, !node].as_slice()));
                cnf.add_clause(Clause::from([fanin1, !node].as_slice()));
            }
        }
        cnf
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
