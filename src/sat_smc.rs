use biodivine_lib_bdd::{Bdd, BddPartialValuation, BddVariableSet};

use crate::sat::{self, Clause, Cube, CNF, DNF};
use crate::{sat::SatSolver, Aig, AigEdge, AigNodeId};
use std::collections::HashMap;

impl Aig {
    fn init_sat_mini(&self) -> sat::minisat::Solver {
        let mut solver = sat::minisat::Solver::new();
        for i in self.nodes_range() {
            if self.nodes[i].is_and() {
                solver.add_and_node(i, self.nodes[i].fanin0(), self.nodes[i].fanin1())
            } else {
                assert!(self.nodes[i].is_cinput());
                solver.add_input_node(i)
            }
        }
        solver
    }

    fn init_sat_glu(&self) -> sat::abc_glucose::Solver {
        let mut solver = sat::abc_glucose::Solver::new();
        for i in self.nodes_range() {
            if self.nodes[i].is_and() {
                solver.add_and_node(i, self.nodes[i].fanin0(), self.nodes[i].fanin1())
            } else {
                assert!(self.nodes[i].is_cinput());
                solver.add_input_node(i)
            }
        }
        solver
    }

    pub fn logic_dnf(&self, logic: AigEdge, latch_transition: &HashMap<AigNodeId, AigEdge>) -> DNF {
        dbg!(logic);
        let mut solver = self.init_sat_glu();
        let mut dnf = DNF::new();
        let mut t = 0;
        while let Some(cex) = solver.solve(&[logic]) {
            let mut clause = Clause::new();
            for lit in cex {
                if latch_transition.contains_key(&lit.node_id()) {
                    clause.push(!*lit);
                }
            }
            t += 1;
            // dbg!(clause.len());
            // dbg!(!clause.clone());
            solver.add_clause(&clause);
            dnf.add_cube(!clause);
        }
        dnf
    }

    pub fn new_smc(&mut self) -> bool {
        let mut reach = self.latch_init_equation();
        let mut frontier = reach;
        let (latch_map, transition) = self.transfer_latch_outputs_into_pinputs();
        let mut rev_latch_map = Vec::new();
        let latch_map = {
            let mut map = HashMap::new();
            for (lout, lin) in latch_map {
                rev_latch_map.push((lin, lout));
                map.insert(lout, lin);
            }
            map
        };
        let mut reach_next = self.migrate_logic(&rev_latch_map, reach);
        let bad = self.bads[0];
        let mut deep = 0;
        loop {
            deep += 1;
            dbg!(deep);
            if self.sat_solver.solve(&[bad, frontier]).is_some() {
                dbg!(deep);
                return false;
            }
            let mut solver = self.init_sat_mini();
            let mut new_frontier = AigEdge::constant_edge(false);
            let mut blocking_clause = 0;
            while let Some(e) = solver.solve(&[frontier, transition, !reach_next]) {
                let mut new_frontier_clause = Vec::new();
                let mut clause = Clause::new();
                for lit in e {
                    if let Some(lin) = latch_map.get(&lit.node_id()) {
                        let mut new_lit = lit.clone();
                        new_lit.set_nodeid(*lin);
                        new_frontier_clause.push(new_lit);
                        clause.push(!*lit);
                    }
                }
                // assert!(clause.len() == 23);
                dbg!(clause.len());
                let new_frontier_clause = self.new_and_nodes(new_frontier_clause);
                new_frontier = self.new_or_node(new_frontier, new_frontier_clause);
                solver.add_clause(&clause);
                blocking_clause += 1;
                dbg!(&blocking_clause);
            }
            let reach_new = self.new_or_node(reach, new_frontier);
            frontier = self.new_and_node(new_frontier, !reach);
            if reach != reach_new {
                reach = reach_new;
                reach_next = self.migrate_logic(&rev_latch_map, reach);
            } else {
                return true;
            }
            dbg!(new_frontier);
        }
    }

    pub fn bad_back_sat_smc(&mut self) -> bool {
        let bads = self.bads[0];
        let mut latch_vec = Vec::new();
        let mut latch_map = HashMap::new();
        for l in self.latchs.iter() {
            latch_vec.push((l.input.into(), l.next));
            assert!(latch_map.insert(l.input, l.next).is_none());
        }
        let init = self.latch_init_equation();
        let mut bads = self.migrate_logic_ttt(&latch_vec, bads);
        let init = self.migrate_logic_ttt(&latch_vec, init);
        let mut frontier = bads;
        let mut deep = 0;
        loop {
            deep += 1;
            dbg!(deep);
            if self.sat_solver.solve(&[bads, init]).is_some() {
                dbg!(deep);
                return false;
            }
            let mut solver = self.init_sat_glu();
            let mut blocking_clause = 0;
            let mut all_blocking_clause = 0;
            let mut new_frontier = AigEdge::constant_edge(false);
            while let Some(e) = solver.solve(&[frontier]) {
                let mut new_frontier_clause = Vec::new();
                let mut clause = Clause::new();
                let mut clause_ttt = Vec::new();
                for lit in e {
                    if let Some(lnext) = latch_map.get(&lit.node_id()) {
                        let new_lit = AigEdge::new(lnext.node_id(), lnext.compl() ^ lit.compl());
                        new_frontier_clause.push(new_lit);
                        clause.push(!*lit);
                    }
                    clause_ttt.push(*lit);
                }
                dbg!(clause.len());
                // dbg!(&clause);
                let value = self.evaluate(&clause_ttt);
                assert!(value[frontier.node_id()].unwrap() ^ frontier.compl());
                let new_frontier_clause = self.new_and_nodes(new_frontier_clause);
                new_frontier = self.new_or_node(new_frontier, new_frontier_clause);
                all_blocking_clause += 1 << (latch_map.len() - clause.len());
                solver.add_clause(&clause);
                blocking_clause += 1;
                dbg!(&blocking_clause);
                dbg!(&all_blocking_clause);
            }
            let bads_new = self.new_or_node(bads, new_frontier);
            frontier = self.new_and_node(new_frontier, !bads);
            if bads != bads_new {
                bads = bads_new;
            } else {
                return true;
            }
        }
    }

    pub fn dnf_to_bdd(&self, dnf: &DNF) -> Bdd {
        let mut latch_to_bdd_id = HashMap::new();
        for i in 0..self.latchs.len() {
            latch_to_bdd_id.insert(self.latchs[i].input, i);
        }
        let mut bad_bdd = Vec::new();
        let vars_set = BddVariableSet::new_anonymous(self.latchs.len() as _);
        let vars = vars_set.variables();
        for c in dnf.iter() {
            let mut cube = Vec::new();
            for l in c.iter() {
                cube.push((vars[latch_to_bdd_id[&l.node_id()]], !l.compl()));
            }
            bad_bdd.push(BddPartialValuation::from_values(&cube));
        }
        vars_set.mk_dnf(&bad_bdd)
    }

    pub fn bdd_to_dnf(&self, bdd: &Bdd) -> DNF {
        let dnf: Vec<Cube> = bdd
            .sat_clauses()
            .map(|v| {
                let cube: Vec<AigEdge> = v
                    .to_values()
                    .iter()
                    .map(|(var, val)| {
                        AigEdge::new(self.latchs[Into::<usize>::into(*var)].input, !val)
                    })
                    .collect();
                cube.into()
            })
            .collect();
        dnf.into()
    }

    pub fn bdd_to_cnf(&self, bdd: &Bdd) -> (CNF, AigEdge) {
        let mut ret = CNF::new();
        let (cnf, logic) = bdd.cnf();
        for clause in cnf {
            let mut c = Clause::new();
            for (var, compl) in clause {
                let id = if var < self.latchs.len() {
                    self.latchs[var].input
                } else {
                    var - self.latchs.len() + self.nodes.len()
                };
                c.push(AigEdge::new(id, compl));
            }
            ret.push(c);
        }
        (ret, logic.into())
    }

    pub fn bad_back_sat_smc_without_aig(&mut self) -> bool {
        let mut latch_transition = HashMap::new();
        let mut init = Cube::new();
        for l in self.latchs.iter() {
            init.push(AigEdge::new(l.input, !l.init));
            assert!(latch_transition.insert(l.input, l.next).is_none());
        }
        let mut bad_dnf = self.logic_dnf(self.bads[0], &latch_transition);
        let mut frontier = bad_dnf.clone();
        let mut deep = 0;
        loop {
            deep += 1;
            dbg!(deep);
            dbg!(bad_dnf.len());
            dbg!(frontier.len());
            let mut solver = self.init_sat_glu();
            let mut new_frontier = DNF::new();
            solver.add_cnf(&!bad_dnf.clone());
            if solver.solve(&init).is_none() {
                return false;
            }
            for cube in frontier.iter() {
                let mut assumptions = cube.clone();
                for lit in assumptions.iter_mut() {
                    let next = latch_transition.get(&lit.node_id()).unwrap();
                    lit.set_nodeid(next.node_id());
                    if next.compl() {
                        *lit = !*lit;
                    }
                }
                while let Some(cex) = solver.solve(&assumptions) {
                    let mut clause = Clause::new();
                    for lit in cex {
                        if latch_transition.contains_key(&lit.node_id()) {
                            clause.push(!*lit);
                        }
                    }
                    // dbg!(clause.len());
                    solver.add_clause(&clause);
                    new_frontier.add_cube(!clause);
                }
            }
            // dbg!(&new_frontier);
            if new_frontier.is_empty() {
                dbg!(deep);
                return true;
            } else {
                bad_dnf = bad_dnf + new_frontier.clone();
                frontier = new_frontier;
            }
        }
    }

    pub fn bad_back_sat_smc_without_aig_with_bdd(&mut self) -> bool {
        let mut latch_transition = HashMap::new();
        let mut init = Cube::new();
        for l in self.latchs.iter() {
            init.push(AigEdge::new(l.input, !l.init));
            assert!(latch_transition.insert(l.input, l.next).is_none());
        }
        let mut bad_dnf = self.logic_dnf(self.bads[0], &latch_transition);
        let mut bad_bdd = self.dnf_to_bdd(&bad_dnf);
        let mut frontier = bad_dnf.clone();
        let mut deep = 0;
        loop {
            deep += 1;
            dbg!(deep);
            dbg!(bad_dnf.len());
            dbg!(frontier.len());
            let mut solver = self.init_sat_glu();
            let mut new_frontier = DNF::new();
            solver.add_cnf(&!bad_dnf.clone());
            if solver.solve(&init).is_none() {
                return false;
            }
            for cube in frontier.iter() {
                let mut assumptions = cube.clone();
                for lit in assumptions.iter_mut() {
                    let next = latch_transition.get(&lit.node_id()).unwrap();
                    lit.set_nodeid(next.node_id());
                    if next.compl() {
                        *lit = !*lit;
                    }
                }
                while let Some(cex) = solver.solve(&assumptions) {
                    let mut clause = Clause::new();
                    for lit in cex {
                        if latch_transition.contains_key(&lit.node_id()) {
                            clause.push(!*lit);
                        }
                    }
                    // dbg!(clause.len());
                    solver.add_clause(&clause);
                    new_frontier.add_cube(!clause);
                }
            }
            // dbg!(&new_frontier);
            if new_frontier.is_empty() {
                dbg!(deep);
                return true;
            } else {
                bad_dnf = bad_dnf + new_frontier.clone();
                dbg!(bad_dnf.len());
                let bad_new_frontier = self.dnf_to_bdd(&new_frontier);
                bad_bdd = bad_bdd.or(&bad_new_frontier);
                dbg!(bad_bdd.size());
                bad_dnf = self.bdd_to_dnf(&bad_bdd);
                dbg!(bad_dnf.len());
                frontier = self.bdd_to_dnf(&bad_new_frontier);
            }
        }
    }
}
