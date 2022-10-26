use crate::{
    sat::SatSolver,
    simulate::{Simulation, SimulationWords, SimulationWordsHash},
    Aig, AigEdge, AigNodeId,
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
        // let mut old_keys = Vec::new();
        // for v in old_map.values() {
        //     for key in &old_keys {
        //         assert_ne!(*key, self.simulation.hash_value(v[0]));
        //     }
        //     old_keys.push(self.simulation.hash_value(v[0]));
        // }
        self.simulation.add_pattern(pattern);
        // let mut keys: Vec<(u128, AigEdge)> = Vec::new();
        // let mut ttt = 0;
        // for v in old_map.values() {
        //     for i in 0..keys.len() {
        //         if keys[i].0 == self.simulation.hash_value(v[0]) {
        //             dbg!(keys[i]);
        //             dbg!(self.simulation.hash_value(v[0]));
        //             dbg!(old_keys[i]);
        //             dbg!(old_keys[ttt]);
        //             dbg!(&self.simulation.simulations()[v[0].node_id()]);
        //             dbg!(&self.simulation.simulations()[keys[i].1.node_id()]);
        //             panic!();
        //         }
        //     }
        //     ttt += 1;
        //     keys.push((self.simulation.hash_value(v[0]), v[0]));
        // }
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

    pub fn new_and_node(
        &mut self,
        solver: &mut SatSolver,
        fanin0: AigEdge,
        fanin1: AigEdge,
        new_node: AigNodeId,
    ) -> AigEdge {
        let sim = self.simulation.sim_value(fanin0) & self.simulation.sim_value(fanin1);
        match self.sim_map.get_mut(&sim.hash_value()) {
            Some(c) => match solver.equivalence_check_xy_z(fanin0, fanin1, c[0]) {
                Some(p) => {
                    self.add_pattern(p);
                    let sim = self.simulation.sim_value(fanin0) & self.simulation.sim_value(fanin1);
                    assert!(!self.sim_map.contains_key(&sim.hash_value()));
                    self.add_new_node(sim, new_node.into());
                    new_node.into()
                }
                None => c[0],
            },
            None => match self.sim_map.get(&!sim.hash_value()) {
                Some(c) => match solver.equivalence_check_xy_z(fanin0, fanin1, !c[0]) {
                    Some(p) => {
                        self.add_pattern(p);
                        let sim =
                            self.simulation.sim_value(fanin0) & self.simulation.sim_value(fanin1);
                        self.add_new_node(sim, new_node.into());
                        new_node.into()
                    }
                    None => !c[0],
                },
                None => {
                    self.add_new_node(sim, new_node.into());
                    new_node.into()
                }
            },
        }
    }
}

impl Aig {
    fn get_candidate(
        &mut self,
        simulation: &Simulation,
    ) -> HashMap<SimulationWordsHash, Vec<AigEdge>> {
        let mut candidate_map: HashMap<SimulationWordsHash, Vec<AigEdge>> = HashMap::new();
        for idx in self.nodes_range_with_true() {
            match candidate_map.get_mut(&simulation.simulations()[idx].hash_value()) {
                Some(candidate) => candidate.push(AigEdge::new(idx, false)),
                None => {
                    match candidate_map.get_mut(&!&simulation.simulations()[idx].hash_value()) {
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
        let mut simulation = self.new_simulation(10000);
        loop {
            let candidates = self.get_candidate(&simulation);
            let mut update = false;
            for candidate in candidates.values() {
                if candidate.len() == 1 {
                    continue;
                }
                for c in &candidate[1..] {
                    if let Some(p) = self.sat_solver.equivalence_check(candidate[0], *c) {
                        assert!(p.len() == self.num_nodes() - 1);
                        simulation.add_pattern(p);
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
    fn test() {
        let mut aig = Aig::from_file("aigs/cec1.aag").unwrap();
        println!("{}", aig);
        aig.fraig();
        println!("{}", aig);
    }
}
