use rand::{thread_rng, Rng};

use crate::{
    sat::SatSolver,
    simulate::{Simulation, SimulationWords, SimulationWordsHash},
    Aig, AigEdge, AigNode, AigNodeId,
};
use std::{collections::HashMap, mem::take, vec};

#[derive(Debug)]
pub struct FrAig {
    simulation: Simulation,
    sim_map: HashMap<SimulationWordsHash, Vec<AigEdge>>,
}

impl FrAig {
    fn add_pattern(&mut self, pattern: Vec<bool>) {
        let old_map = take(&mut self.sim_map);
        self.simulation.add_pattern(pattern);
        for (_, c) in old_map {
            assert!(self
                .sim_map
                .insert(self.simulation.hash_value(c[0]), c)
                .is_none());
        }
    }

    fn add_new_node(&mut self, sim: SimulationWords, edge: AigEdge) {
        assert!(self.sim_map.insert(sim.hash_value(), vec![edge]).is_none());
        self.simulation.add_node(sim);
    }

    pub fn new_input_node(&mut self, node: AigNodeId) {
        let mut sim = SimulationWords::new(self.simulation.nbit());
        while self.sim_map.contains_key(&sim.hash_value()) {
            sim = SimulationWords::new(self.simulation.nbit());
        }
        assert!(self
            .sim_map
            .insert(sim.hash_value(), vec![node.into()])
            .is_none());
        self.simulation.add_node(sim);
    }
}

impl Aig {
    pub fn new_and_node_inner(
        &mut self,
        fanin0: AigEdge,
        fanin1: AigEdge,
        new_node: AigNodeId,
    ) -> AigEdge {
        let fraig = self.fraig.as_mut().unwrap();
        let sim = fraig.simulation.sim_value(fanin0) & fraig.simulation.sim_value(fanin1);
        match fraig.sim_map.get_mut(&sim.hash_value()) {
            Some(c) => match self.sat_solver.equivalence_check_xy_z(fanin0, fanin1, c[0]) {
                Some(s) => {
                    fraig.add_pattern(Self::gen_pattern(&self.nodes, s));
                    let sim =
                        fraig.simulation.sim_value(fanin0) & fraig.simulation.sim_value(fanin1);
                    assert!(!fraig.sim_map.contains_key(&sim.hash_value()));
                    fraig.add_new_node(sim, new_node.into());
                    new_node.into()
                }
                None => c[0],
            },
            None => match fraig.sim_map.get(&!sim.hash_value()) {
                Some(c) => match self
                    .sat_solver
                    .equivalence_check_xy_z(fanin0, fanin1, !c[0])
                {
                    Some(s) => {
                        fraig.add_pattern(Self::gen_pattern(&self.nodes, s));
                        let sim =
                            fraig.simulation.sim_value(fanin0) & fraig.simulation.sim_value(fanin1);
                        fraig.add_new_node(sim, new_node.into());
                        new_node.into()
                    }
                    None => !c[0],
                },
                None => {
                    fraig.add_new_node(sim, new_node.into());
                    new_node.into()
                }
            },
        }
    }
}

impl Aig {
    fn gen_pattern(nodes: &[AigNode], s: &[AigEdge]) -> Vec<bool> {
        let mut r = thread_rng();
        let mut flags = vec![false; nodes.len() - 1];
        let mut ret = vec![false; nodes.len() - 1];
        for e in s {
            ret[e.node_id() - 1] = !e.compl();
            flags[e.node_id() - 1] = true;
        }
        for i in 1..nodes.len() {
            if !flags[i - 1] {
                flags[i - 1] = true;
                if nodes[i].is_and() {
                    let fanin0 = nodes[i].fanin0();
                    let fanin1 = nodes[i].fanin1();
                    let v0 = ret[fanin0.node_id() - 1] ^ fanin0.compl();
                    let v1 = ret[fanin1.node_id() - 1] ^ fanin1.compl();
                    ret[i - 1] = v0 & v1;
                } else {
                    ret[i - 1] = r.gen();
                }
            }
        }
        ret
    }

    fn get_candidate(
        &mut self,
        simulation: &Simulation,
    ) -> HashMap<SimulationWordsHash, Vec<AigEdge>> {
        let mut candidate_map: HashMap<SimulationWordsHash, Vec<AigEdge>> = HashMap::new();
        for idx in self.nodes_range_with_true() {
            match candidate_map.get_mut(&simulation.simulations()[idx].hash_value()) {
                Some(candidate) => candidate.push(AigEdge::new(idx, false)),
                None => {
                    match candidate_map
                        .get_mut(&(!simulation.simulations()[idx].clone()).hash_value())
                    {
                        Some(candidate) => candidate.push(AigEdge::new(idx, true)),
                        None => {
                            candidate_map.insert(
                                simulation.simulations()[idx].hash_value(),
                                vec![AigEdge::new(idx, false)],
                            );
                        }
                    }
                }
            }
        }
        candidate_map
    }

    pub fn fraig(&mut self) {
        assert!(self.fraig.is_none());
        let mut simulation = self.new_simulation(1000);
        loop {
            let candidates = self.get_candidate(&simulation);
            let mut update = false;
            for candidate in candidates.values() {
                if candidate.len() == 1 {
                    continue;
                }
                for c in &candidate[1..] {
                    if let Some(s) = self.sat_solver.equivalence_check(candidate[0], *c) {
                        simulation.add_pattern(Self::gen_pattern(&self.nodes, s));
                        update = true;
                    }
                }
            }
            if !update {
                for (k, candidate) in &candidates {
                    assert_eq!(*k, simulation.hash_value(candidate[0]));
                    for c in &candidate[1..] {
                        assert_eq!(*k, simulation.hash_value(*c));
                        self.merge_fe_node(*c, candidate[0]);
                    }
                }

                self.fraig = Some(FrAig {
                    simulation,
                    sim_map: candidates,
                });
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Aig;
    #[test]
    fn test1() {
        let mut aig = Aig::from_file("aigs/cec1.aag").unwrap();
        aig.fraig();
        assert_eq!(aig.fraig.unwrap().sim_map.keys().len(), 6);
    }

    #[test]
    fn test2() {
        let mut aig = Aig::from_file("aigs/cec2.aag").unwrap();
        aig.fraig();
        assert_eq!(aig.fraig.unwrap().sim_map.keys().len(), 8);
    }
}
