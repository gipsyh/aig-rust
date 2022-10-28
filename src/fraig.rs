use crate::{
    simulate::{Simulation, SimulationWords, SimulationWordsHash},
    Aig, AigEdge, AigNode, AigNodeId,
};
use rand::{thread_rng, Rng};
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
            let (hash_value, _) = self.simulation.abs_hash_value(c[0]);
            assert!(self.sim_map.insert(hash_value, c).is_none());
        }
    }

    fn add_new_node(&mut self, sim: SimulationWords, mut edge: AigEdge) {
        if sim.compl() {
            edge = !edge;
        }
        assert!(self
            .sim_map
            .insert(sim.abs_hash_value(), vec![edge])
            .is_none());
        self.simulation.add_node(sim);
    }

    pub fn new_input_node(&mut self, node: AigNodeId) {
        let mut sim = SimulationWords::new(self.simulation.nbit());
        while self.sim_map.contains_key(&sim.abs_hash_value()) {
            sim = SimulationWords::new(self.simulation.nbit());
        }
        let edge = if sim.compl() {
            AigEdge::new(node, true)
        } else {
            AigEdge::new(node, false)
        };
        assert!(self
            .sim_map
            .insert(sim.abs_hash_value(), vec![edge])
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
        let sim = fraig.simulation.sim_and(fanin0, fanin1);
        match fraig.sim_map.get(&sim.abs_hash_value()) {
            Some(c) => {
                let can = if sim.compl() { !c[0] } else { c[0] };
                match self.sat_solver.equivalence_check_xy_z(fanin0, fanin1, can) {
                    Some(s) => {
                        fraig.add_pattern(Self::gen_pattern(&self.nodes, s));
                        let sim = fraig.simulation.sim_and(fanin0, fanin1);
                        fraig.add_new_node(sim, new_node.into());
                        new_node.into()
                    }
                    None => can,
                }
            }
            None => {
                fraig.add_new_node(sim, new_node.into());
                new_node.into()
            }
        }
    }
}

impl Aig {
    fn gen_pattern(nodes: &[AigNode], s: &[AigEdge]) -> Vec<bool> {
        let mut r = thread_rng();
        let mut flags = vec![false; nodes.len()];
        let mut ret = vec![false; nodes.len()];
        ret[0] = true;
        for e in s {
            ret[e.node_id()] = !e.compl();
            flags[e.node_id()] = true;
        }
        for i in 1..nodes.len() {
            if !flags[i] {
                flags[i] = true;
                if nodes[i].is_and() {
                    let fanin0 = nodes[i].fanin0();
                    let fanin1 = nodes[i].fanin1();
                    let v0 = ret[fanin0.node_id()] ^ fanin0.compl();
                    let v1 = ret[fanin1.node_id()] ^ fanin1.compl();
                    ret[i] = v0 & v1;
                } else {
                    ret[i] = r.gen();
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
            match candidate_map.get_mut(&simulation[idx].abs_hash_value()) {
                Some(candidate) => {
                    if simulation[idx].compl() {
                        candidate.push(AigEdge::new(idx, true))
                    } else {
                        candidate.push(AigEdge::new(idx, false))
                    }
                }
                None => {
                    let edge = if simulation[idx].compl() {
                        AigEdge::new(idx, true)
                    } else {
                        AigEdge::new(idx, false)
                    };
                    assert!(candidate_map
                        .insert(simulation[idx].abs_hash_value(), vec![edge],)
                        .is_none());
                }
            }
        }
        candidate_map
    }

    pub fn fraig(&mut self) {
        assert!(self.fraig.is_none());
        let mut simulation = self.new_simulation(64);
        loop {
            let candidates = self.get_candidate(&simulation);
            dbg!(candidates.keys().count());
            let mut update = false;
            for candidate in candidates.values() {
                if candidate.len() == 1 {
                    continue;
                }
                for c in &candidate[1..] {
                    if let Some(s) = self.sat_solver.equivalence_check(candidate[0], *c) {
                        // dbg!(s);
                        simulation.add_pattern(Self::gen_pattern(&self.nodes, s));
                        update = true;
                    }
                }
            }
            if !update {
                for (k, candidate) in &candidates {
                    assert_eq!(*k, simulation.abs_hash_value(candidate[0]).0);
                    for c in &candidate[1..] {
                        assert_eq!(*k, simulation.abs_hash_value(*c).0);
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
