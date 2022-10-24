use crate::{
    simulate::{Simulation, SimulationWords},
    Aig, AigNodeId,
};
use std::collections::HashMap;

impl Aig {
    fn get_candidate(&mut self, simulation: &Simulation) -> Vec<Vec<AigNodeId>> {
        let mut candidate_map: HashMap<SimulationWords, Vec<usize>> = HashMap::new();
        for idx in self.nodes_range() {
            match candidate_map.get_mut(&simulation.simulations()[idx]) {
                Some(candidate) => candidate.push(idx),
                None => {
                    candidate_map.insert(simulation.simulations()[idx].clone(), vec![idx]);
                }
            }
        }
        let mut ret = Vec::new();
        for (_, candidate) in candidate_map {
            ret.push(candidate)
        }
        ret
    }

    pub fn fraig(&mut self) {
        let mut simulation = self.new_simulation(100);
        dbg!(self.num_nodes());
        loop {
            let candidates = self.get_candidate(&simulation);
            dbg!(candidates.len());
            let mut update = false;
            for candidate in &candidates {
                if candidate.len() == 1 {
                    continue;
                }
                for c in &candidate[1..] {
                    if let Some(p) = self
                        .sat_solver
                        .equivalence_check(candidate[0].into(), (*c).into())
                    {
                        assert!(p.len() == self.num_nodes() - 1);
                        self.simulation_add_pattern(&mut simulation, p);
                        update = true;
                    }
                }
            }
            if !update {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Aig;
    #[test]
    fn test() {
        let mut aig = Aig::from_file("aigs/i10.aag").unwrap();
        println!("{}", aig);
        aig.fraig();
    }
}
