use crate::sat::minisat;
use crate::sat::SatSolver;
use crate::{Aig, AigEdge, AigNodeId};
use std::collections::HashMap;

struct DualVariables {
    origin_to_duals: HashMap<AigNodeId, (AigNodeId, AigNodeId)>,
    duals_to_origin: HashMap<AigNodeId, AigNodeId>,
    duals_to_duals: HashMap<AigNodeId, AigNodeId>,
}

impl DualVariables {
    fn new(duals: Vec<(AigNodeId, AigNodeId, AigNodeId)>) -> Self {
        let mut origin_to_duals = HashMap::new();
        let mut duals_to_origin = HashMap::new();
        let mut duals_to_duals = HashMap::new();
        for (o, pos, neg) in duals {
            origin_to_duals.insert(o, (pos, neg));
            duals_to_origin.insert(pos, o);
            duals_to_origin.insert(neg, o);
            duals_to_duals.insert(neg, pos);
            duals_to_duals.insert(pos, neg);
        }
        Self {
            origin_to_duals,
            duals_to_origin,
            duals_to_duals,
        }
    }

    fn dual_to_edge(&self, node: AigNodeId) -> Option<AigEdge> {
        self.duals_to_origin.get(&node).map(|o| {
            let (pos, neg) = self.origin_to_duals.get(o).unwrap();
            assert!(*pos == node || *neg == node);
            AigEdge::new(*o, node != *pos)
        })
    }

    fn dual(&self, dual: AigNodeId) -> Option<AigNodeId> {
        self.duals_to_duals.get(&dual).map(|dual| *dual)
    }
}

impl Aig {
    fn create_sort_gate(&mut self, input0: AigEdge, input1: AigEdge) -> (AigEdge, AigEdge) {
        (
            self.new_or_node(input0, input1),
            self.new_and_node(input0, input1),
        )
    }

    fn create_sort_networks(&mut self, inputs: &[AigEdge]) -> Vec<AigEdge> {
        let mut ret = inputs.to_vec();
        for round in 0..inputs.len() {
            dbg!(&ret);
            let mut new = Vec::new();
            if round % 2 > 0 {
                for i in (0..ret.len() - 1).step_by(2) {
                    let (out0, out1) = self.create_sort_gate(ret[i], ret[i + 1]);
                    new.push(out0);
                    new.push(out1);
                }
                if inputs.len() % 2 > 0 {
                    new.push(*ret.last().unwrap());
                }
            } else {
                new.push(ret[0]);
                for i in (1..ret.len() - 1).step_by(2) {
                    let (out0, out1) = self.create_sort_gate(ret[i], ret[i + 1]);
                    new.push(out0);
                    new.push(out1);
                }
                if inputs.len() % 2 == 0 {
                    new.push(*ret.last().unwrap());
                }
            }
            ret = new;
            assert!(ret.len() == inputs.len());
        }
        ret
    }

    fn edge_to_cnf(&self, edge: AigEdge) -> Vec<Vec<AigEdge>> {
        let mut ret = Vec::new();
        let flag = self.fanin_logic_cone([&edge]);
        for i in self.nodes_range_with_true() {
            if flag[i] {
                if self.nodes[i].is_and() {
                    let fanin0 = self.nodes[i].fanin0();
                    let fanin1 = self.nodes[i].fanin1();
                    let node = AigEdge::new(i, false);
                    ret.push(vec![!fanin0, !fanin1, node]);
                    ret.push(vec![fanin0, !node]);
                    ret.push(vec![fanin1, !node]);
                }
            }
        }
        ret.push(vec![edge]);
        ret
    }
}

impl Aig {
    fn init_sat_with_dual(
        &mut self,
        transitions: Vec<(AigEdge, AigEdge)>,
        latch_map: &HashMap<usize, usize>,
    ) -> (minisat::Solver, DualVariables, Vec<AigEdge>) {
        let mut solver = minisat::Solver::new();
        let mut duals = Vec::new();
        for i in self.nodes_range() {
            if self.nodes[i].is_and() {
                duals.push(solver.add_and_node_with_dual(
                    i,
                    self.nodes[i].fanin0(),
                    self.nodes[i].fanin1(),
                ))
            } else {
                assert!(self.nodes[i].is_cinput());
                duals.push(solver.add_input_node_with_dual(i))
            }
        }
        for (pos, neg) in duals.iter() {
            solver.add_clause(&[AigEdge::new(*pos, true), AigEdge::new(*neg, true)])
        }
        for (l, r) in transitions {
            solver.add_equal_node_with_dual(l, r);
        }
        let mut tmp = Vec::new();
        let mut duals_or = Vec::new();
        // dbg!(&duals[1-1]);
        // dbg!(&duals[118-1]);
        // panic!();
        for (l, _) in latch_map.iter() {
            let pos = duals[*l - 1].0;
            let neg = duals[*l - 1].1;
            // let or_node = solver.new_variable();
            // let or_node_edge: AigEdge = or_node.into();
            // let pos_edge: AigEdge = pos.into();
            // let neg_edge: AigEdge = neg.into();
            // solver.add_clause(&[or_node_edge, !pos_edge]);
            // solver.add_clause(&[or_node_edge, !neg_edge]);
            // solver.add_clause(&[!or_node_edge, neg_edge, pos_edge]);
            // duals_or.push(or_node.into());
            tmp.push((*l, pos, neg))
        }
        dbg!(&tmp);
        (solver, DualVariables::new(tmp), duals_or)
    }
    fn init_sat(&self) -> minisat::Solver {
        let mut solver = minisat::Solver::new();
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

    pub fn new_smc_with_dual(&mut self) -> bool {
        let mut reach = self.latch_init_equation();
        let mut frontier = reach;
        let (latch_map, transition) = self.transfer_latch_outputs_into_pinputs_with_dual();
        let latch_map = {
            let mut map = HashMap::new();
            for (lout, lin) in latch_map {
                map.insert(lout, lin);
            }
            map
        };
        // let bad = self.bads[0];
        let fanin = self.fanin_logic_cone([&AigEdge::new(465, false)]);
        for i in 0..fanin.len() {
            if fanin[i] {
                println!("{}", self.nodes[i]);
            }
        }
        panic!();
        println!("{}", self);
        let mut deep = 0;
        loop {
            deep += 1;
            dbg!(deep);
            // if self.sat_solver.solve(&[bad, frontier]).is_some() {
            //     dbg!(deep);
            //     return false;
            // }
            let (mut solver, duals, duals_or) =
                self.init_sat_with_dual(transition.clone(), &latch_map);
            let mut new_frontier = AigEdge::constant_edge(false);
            let mut blocking_clause = 0;
            let mut all_blocking_clause = 0;
            // let mut dyn_sort: Vec<AigEdge> = sort_net.iter().map(|out| !*out).collect();
            // dyn_sort.push(frontier);
            // for i in 0..sort_net.len() {
            // dyn_sort[i] = !dyn_sort[i];
            // dbg!(&duals_or);
            while let Some(e) = solver.solve_with_dual(&[frontier], &[]) {
                let mut new_frontier_clause = Vec::new();
                let mut clause = Vec::new();
                for lit in e {
                    if lit.node_id() == 1 || lit.node_id() == 2 {
                        dbg!(lit);
                    }
                    if lit.node_id() == 235 || lit.node_id() == 236 {
                        dbg!(lit);
                    }
                    if !lit.compl() {
                        if let Some(dual) = duals.dual(lit.node_id()) {
                            let mut new_lit = duals.dual_to_edge(lit.node_id()).unwrap();
                            new_lit.set_nodeid(*latch_map.get(&new_lit.node_id()).unwrap());
                            new_frontier_clause.push(new_lit);
                            clause.push(AigEdge::new(dual, false));
                        }
                    }
                }
                let new_frontier_clause = self.new_and_nodes(new_frontier_clause);
                new_frontier = self.new_or_node(new_frontier, new_frontier_clause);
                // assert!(i + 1 == clause.len());
                dbg!(&clause);
                dbg!(clause.len());
                solver.add_clause(&clause);
                blocking_clause += 1;
                all_blocking_clause += 1 << (transition.len() - clause.len());
                dbg!(&blocking_clause);
                dbg!(&all_blocking_clause);
            }
            // }
            assert!(deep < 1);
            let reach_new = self.new_or_node(reach, new_frontier);
            frontier = self.new_and_node(new_frontier, !reach);
            if reach != reach_new {
                reach = reach_new
            } else {
                return true;
            }
            dbg!(new_frontier);
        }
    }

    pub fn new_smc(&mut self) -> bool {
        let mut reach = self.latch_init_equation();
        let mut frontier = reach;
        let (latch_map, mut transition) = self.transfer_latch_outputs_into_pinputs();
        let latch_map = {
            let mut map = HashMap::new();
            for (lout, lin) in latch_map {
                map.insert(lout, lin);
            }
            map
        };
        let mut bad = self.bads[0];
        let mut deep = 0;
        loop {
            deep += 1;
            dbg!(deep);
            if self.sat_solver.solve(&[bad, frontier]).is_some() {
                dbg!(deep);
                return false;
            }
            let mut solver = self.init_sat();
            let mut new_frontier = AigEdge::constant_edge(false);
            let mut blocking_clause = 0;
            while let Some(e) = solver.solve(&[frontier, transition]) {
                let mut new_frontier_clause = Vec::new();
                let mut clause = Vec::new();
                for lit in e {
                    if let Some(lin) = latch_map.get(&lit.node_id()) {
                        let mut new_lit = lit.clone();
                        new_lit.set_nodeid(*lin);
                        new_frontier_clause.push(new_lit);
                        clause.push(!*lit);
                    }
                }
                let new_frontier_clause = self.new_and_nodes(new_frontier_clause);
                new_frontier = self.new_or_node(new_frontier, new_frontier_clause);
                dbg!(&clause);
                dbg!(&clause.len());
                solver.add_clause(&clause);
                blocking_clause += 1;
                dbg!(&blocking_clause);
            }
            assert!(deep < 1);
            let reach_new = self.new_or_node(reach, new_frontier);
            frontier = self.new_and_node(new_frontier, !reach);
            if reach != reach_new {
                reach = reach_new
            } else {
                return true;
            }
            dbg!(new_frontier);
        }
    }
}
