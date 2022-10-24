use crate::{
    simulate::{Simulation, SimulationWords},
    Aig, AigEdge,
};
use std::collections::HashMap;

#[derive(Debug)]
pub struct FrAig {
    simulation: Simulation,
    sim_map: HashMap<SimulationWords, Vec<AigEdge>>,
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
                        self.simulation_add_pattern(&mut simulation, p);
                        update = true;
                    }
                }
            }
            if !update {
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
    }
}
