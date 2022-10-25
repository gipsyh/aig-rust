use crate::{
    sat::SatSolver,
    simulate::{Simulation, SimulationWords},
    Aig, AigEdge, AigNodeId,
};
use std::{collections::HashMap, vec};

#[derive(Debug)]
pub struct FrAig {
    simulation: Simulation,
    sim_map: HashMap<SimulationWords, Vec<AigEdge>>,
}

impl FrAig {
    fn add_pattern(&mut self, pattern: Vec<bool>) {
        self.simulation.add_pattern(pattern);
        let mut new_map: HashMap<SimulationWords, Vec<AigEdge>> = HashMap::new();
        for v in self.sim_map.values() {
            for e in v {
                let key = self.simulation.sim_value(*e);
                match new_map.get_mut(&key) {
                    Some(ev) => ev.push(*e),
                    None => {
                        assert!(!new_map.contains_key(&!key.clone()));
                        new_map.insert(key, vec![*e]);
                    }
                }
            }
        }
        self.sim_map = new_map;
    }

    fn add_new_node(&mut self, sim: SimulationWords, edge: AigEdge) {
        self.simulation.add_node(sim.clone());
        assert!(self.sim_map.insert(sim, vec![edge]).is_none());
    }

    pub fn new_input_node(&mut self, node: AigNodeId) {
        let mut sim = SimulationWords::new(self.simulation.nwords());
        while self.sim_map.contains_key(&sim) {
            sim = SimulationWords::new(self.simulation.nwords());
        }
        self.sim_map.insert(sim.clone(), vec![node.into()]);
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
        match self.sim_map.get_mut(&sim) {
            Some(c) => match solver.equivalence_check_xy_z(fanin0, fanin1, c[0]) {
                Some(p) => {
                    self.add_pattern(p);
                    let sim = self.simulation.sim_value(fanin0) & self.simulation.sim_value(fanin1);
                    self.add_new_node(sim, new_node.into());
                    new_node.into()
                }
                None => c[0],
            },
            None => match self.sim_map.get(&!sim.clone()) {
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
    fn get_candidate(&mut self, simulation: &Simulation) -> HashMap<SimulationWords, Vec<AigEdge>> {
        let mut candidate_map: HashMap<SimulationWords, Vec<AigEdge>> = HashMap::new();
        for idx in self.nodes_range_with_true() {
            match candidate_map.get_mut(&simulation.simulations()[idx]) {
                Some(candidate) => candidate.push(AigEdge::new(idx, false)),
                None => match candidate_map.get_mut(&!simulation.simulations()[idx].clone()) {
                    Some(candidate) => candidate.push(AigEdge::new(idx, true)),
                    None => {
                        candidate_map.insert(
                            simulation.simulations()[idx].clone(),
                            vec![AigEdge::new(idx, false)],
                        );
                    }
                },
            }
        }
        candidate_map
    }

    pub fn fraig(&mut self) {
        assert!(self.fraig.is_none());
        let mut simulation = self.new_simulation(100);
        dbg!(self.num_nodes());
        loop {
            let candidates = self.get_candidate(&simulation);
            dbg!(&candidates.len());
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
                for candidate in candidates.values() {
                    for c in &candidate[1..] {
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
