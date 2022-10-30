use crate::{
    sat::SatSolver,
    simulate::{
        Simulation, SimulationWord, SimulationWords, SimulationWordsHash, SIMULATION_TRUE_WORD,
    },
    Aig, AigEdge, AigNode, AigNodeId,
};
use rand::{thread_rng, Rng};
use std::{collections::HashMap, mem::take, vec};

#[derive(Debug, Clone)]
pub struct FrAig {
    simulation: Simulation,
    sim_map: HashMap<SimulationWordsHash, Vec<AigEdge>>,
    lazy_cex: Vec<Vec<AigEdge>>,
}

impl FrAig {
    fn generate_words_from_pattern(
        pattern: Vec<Vec<AigEdge>>,
        nodes: &[AigNode],
    ) -> Vec<SimulationWord> {
        let mut rng = thread_rng();
        let mut ret = Vec::new();
        ret.push(SIMULATION_TRUE_WORD);
        for node in nodes.iter().skip(1) {
            if node.is_and() {
                let fanin0 = node.fanin0();
                let fanin1 = node.fanin1();
                let v0 = if fanin0.compl() {
                    !ret[fanin0.node_id()]
                } else {
                    ret[fanin0.node_id()]
                };
                let v1 = if fanin1.compl() {
                    !ret[fanin1.node_id()]
                } else {
                    ret[fanin1.node_id()]
                };
                ret.push(v0 & v1);
            } else {
                ret.push(rng.gen())
            }
        }
        for (nbit, p) in pattern.iter().enumerate().take(pattern.len()) {
            for e in p {
                if e.compl() {
                    ret[e.node_id()] &= !(1 << nbit);
                } else {
                    ret[e.node_id()] |= 1 << nbit;
                }
            }
        }
        for i in 1..nodes.len() {
            if nodes[i].is_and() {
                let fanin0 = nodes[i].fanin0();
                let fanin1 = nodes[i].fanin1();
                let v0 = if fanin0.compl() {
                    !ret[fanin0.node_id()]
                } else {
                    ret[fanin0.node_id()]
                };
                let v1 = if fanin1.compl() {
                    !ret[fanin1.node_id()]
                } else {
                    ret[fanin1.node_id()]
                };
                ret[i] = v0 & v1;
            }
        }
        ret
    }

    fn submit_lazy(&mut self, nodes: &[AigNode]) {
        let words = Self::generate_words_from_pattern(take(&mut self.lazy_cex), nodes);
        self.simulation.add_words(words);
        let old_map = take(&mut self.sim_map);
        for (_, rep_lazys) in old_map {
            for rep_lazy in rep_lazys {
                let (hash_value, compl) = self.simulation.abs_hash_value(rep_lazy);
                assert!(!compl);
                assert!(self.sim_map.insert(hash_value, vec![rep_lazy]).is_none());
            }
        }
    }

    fn add_pattern(&mut self, nodes: &[AigNode], pattern: &[AigEdge]) {
        self.lazy_cex.push(pattern.to_vec());
        if self.lazy_cex.len() == SimulationWord::BITS as usize {
            self.submit_lazy(nodes)
        }
    }

    pub fn new_input_node(&mut self, node: AigNodeId) {
        assert_eq!(self.simulation.num_nodes(), node);
        let mut sim = SimulationWords::new(self.simulation.nbit());
        while self.sim_map.contains_key(&sim.abs_hash_value()) {
            sim = SimulationWords::new(self.simulation.nbit());
        }
        let edge = AigEdge::new(node, sim.compl());
        assert!(self
            .sim_map
            .insert(sim.abs_hash_value(), vec![edge])
            .is_none());
        self.simulation.add_node(sim);
    }

    pub fn new_and_node(
        &mut self,
        nodes: &[AigNode],
        solver: &mut dyn SatSolver,
        fanin0: AigEdge,
        fanin1: AigEdge,
        new_node: AigNodeId,
    ) -> Option<AigEdge> {
        let sim = self.simulation.sim_and(fanin0, fanin1);
        match self.sim_map.get(&sim.abs_hash_value()) {
            Some(cans) => {
                let cans = cans.clone();
                for can in cans {
                    let can = if sim.compl() { !can } else { can };
                    match solver.equivalence_check_xy_z(fanin0, fanin1, can) {
                        Some(s) => self.add_pattern(nodes, s),
                        None => {
                            return Some(can);
                        }
                    }
                }

                let sim = if sim.nbit() != self.simulation.nbit() {
                    self.simulation.sim_and(fanin0, fanin1)
                } else {
                    sim
                };
                let new_edge = AigEdge::new(self.simulation.num_nodes(), sim.compl());
                match self.sim_map.get_mut(&sim.abs_hash_value()) {
                    Some(can) => can.push(new_edge),
                    None => assert!(self
                        .sim_map
                        .insert(sim.abs_hash_value(), vec![new_edge])
                        .is_none()),
                };
                self.simulation.add_node(sim);
                None
            }
            None => {
                assert_eq!(self.simulation.num_nodes(), new_node);
                let new_edge = AigEdge::new(self.simulation.num_nodes(), sim.compl());
                assert!(self
                    .sim_map
                    .insert(sim.abs_hash_value(), vec![new_edge])
                    .is_none());
                self.simulation.add_node(sim);
                None
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
        let mut simulation = self.new_simulation(2);
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
                        simulation.add_bits(Self::gen_pattern(&self.nodes, s));
                        update = true;
                    }
                }
            }
            if !update {
                let mut sim_map = HashMap::new();
                for (k, candidate) in &candidates {
                    assert_eq!(*k, simulation.abs_hash_value(candidate[0]).0);
                    assert!(sim_map.insert(*k, vec![candidate[0]]).is_none());
                    for c in &candidate[1..] {
                        assert_eq!(*k, simulation.abs_hash_value(*c).0);
                        self.merge_fe_node(*c, candidate[0]);
                    }
                }
                self.fraig = Some(FrAig {
                    simulation,
                    sim_map,
                    lazy_cex: Vec::new(),
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
