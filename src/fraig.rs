use crate::{
    sat::SatSolver,
    simulate::{Simulation, SimulationWord, SimulationWords, SimulationWordsHash},
    symbolic_mc::{
        TOTAL_ADD_PATTERN, TOTAL_BUG, TOTAL_FRAIG_ADD_SAT, TOTAL_RESIM, TOTAL_SIMAND,
        TOTAL_SIMAND_NOSAT_INSERT, TOTAL_SIMAND_SAT_INSERT,
    },
    Aig, AigEdge, AigNode, AigNodeId,
};
use rand::{thread_rng, Rng};
use std::{
    collections::HashMap,
    mem::{replace, take},
    vec,
};

#[derive(Debug, Clone)]
pub struct FrAig {
    simulation: Simulation,
    sim_map: HashMap<SimulationWordsHash, Vec<AigEdge>>,
    lazy_cex: Vec<SimulationWord>,
    ncex: usize,
}

impl FrAig {
    fn default_lazy_cexs(&self) -> Vec<SimulationWord> {
        let mut ret = Vec::with_capacity(self.simulation.num_nodes());
        let ret_remains = ret.spare_capacity_mut();
        for (i, ret_remain) in ret_remains.iter_mut().enumerate() {
            ret_remain.write(self.simulation[i][0]);
        }
        unsafe { ret.set_len(self.simulation.num_nodes()) }
        ret
    }

    fn submit_lazy(&mut self) {
        let default_lazy = self.default_lazy_cexs();
        self.simulation
            .add_words(replace(&mut self.lazy_cex, default_lazy));
        self.ncex = 0;
        let old_map = replace(
            &mut self.sim_map,
            HashMap::with_capacity(self.simulation.num_nodes()),
        );
        for (_, rep_lazys) in old_map {
            for rep_lazy in rep_lazys.iter() {
                let (hash_value, compl) = self.simulation.abs_hash_value(*rep_lazy);
                assert!(!compl);
                assert!(self.sim_map.insert(hash_value, vec![*rep_lazy]).is_none());
            }
        }
    }

    fn lazy_resimulate(&mut self, nodes: &[AigNode]) {
        for node in nodes.iter().skip(1) {
            if node.is_and() {
                let fanin0 = node.fanin0();
                let fanin1 = node.fanin1();
                let v0 = if fanin0.compl() {
                    !self.lazy_cex[fanin0.node_id()]
                } else {
                    self.lazy_cex[fanin0.node_id()]
                };
                let v1 = if fanin1.compl() {
                    !self.lazy_cex[fanin1.node_id()]
                } else {
                    self.lazy_cex[fanin1.node_id()]
                };
                self.lazy_cex[node.node_id()] = v0 & v1;
            }
        }
    }

    fn add_pattern(&mut self, nodes: &[AigNode], pattern: &[AigEdge]) {
        unsafe { TOTAL_ADD_PATTERN += 1 };
        for e in pattern {
            if e.compl() {
                self.lazy_cex[e.node_id()] &= !(1 << self.ncex);
            } else {
                self.lazy_cex[e.node_id()] |= 1 << self.ncex;
            }
        }
        self.lazy_resimulate(nodes);
        self.ncex += 1;
        if self.ncex == SimulationWord::BITS as usize {
            self.submit_lazy();
        }
    }

    pub fn new_input_node(&mut self, node: AigNodeId) {
        assert_eq!(self.simulation.num_nodes(), node);
        assert_eq!(self.lazy_cex.len(), node);
        let mut sim = SimulationWords::new(self.simulation.nword());
        while self.sim_map.contains_key(&sim.abs_hash_value()) {
            sim = SimulationWords::new(self.simulation.nword());
        }
        let edge = AigEdge::new(node, sim.compl());
        assert!(self
            .sim_map
            .insert(sim.abs_hash_value(), vec![edge])
            .is_none());
        self.simulation.add_node(sim);
        let mut rng = thread_rng();
        self.lazy_cex.push(rng.gen());
    }

    #[inline]
    pub fn new_and_node(
        &mut self,
        nodes: &[AigNode],
        solver: &mut dyn SatSolver,
        fanin0: AigEdge,
        fanin1: AigEdge,
    ) -> Option<AigEdge> {
        unsafe { TOTAL_SIMAND += 1 };
        let sim = self.simulation.sim_and(fanin0, fanin1);
        let lazy_value_closure = |e: AigEdge, lazy: &Vec<SimulationWord>| {
            if e.compl() {
                !lazy[e.node_id()]
            } else {
                lazy[e.node_id()]
            }
        };
        let new_and_lazy_closure = |lazy: &Vec<SimulationWord>| {
            let fanin0_lazy = lazy_value_closure(fanin0, lazy);
            let fanin1_lazy = lazy_value_closure(fanin1, lazy);
            fanin0_lazy & fanin1_lazy
        };
        match self.sim_map.get(&sim.abs_hash_value()) {
            Some(cans) => {
                let cans = cans.clone();
                for can in cans {
                    let can = if sim.compl() { !can } else { can };
                    if lazy_value_closure(can, &self.lazy_cex)
                        != new_and_lazy_closure(&self.lazy_cex)
                    {
                        assert!(
                            lazy_value_closure(!can, &self.lazy_cex)
                                != new_and_lazy_closure(&self.lazy_cex)
                        );
                        continue;
                    }
                    unsafe { TOTAL_FRAIG_ADD_SAT += 1 };
                    match solver.equivalence_check_xy_z(fanin0, fanin1, can) {
                        Some(s) => self.add_pattern(nodes, s),
                        None => {
                            return Some(can);
                        }
                    }
                }

                let sim = if sim.nword() != self.simulation.nword() {
                    unsafe { TOTAL_RESIM += 1 };
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
                unsafe { TOTAL_SIMAND_SAT_INSERT += 1 };
                self.simulation.add_node(sim);
                self.lazy_cex.push(new_and_lazy_closure(&self.lazy_cex));
                None
            }
            None => {
                let new_edge = AigEdge::new(self.simulation.num_nodes(), sim.compl());
                unsafe { TOTAL_SIMAND_NOSAT_INSERT += 1 };
                assert!(self
                    .sim_map
                    .insert(sim.abs_hash_value(), vec![new_edge])
                    .is_none());
                self.simulation.add_node(sim);
                self.lazy_cex.push(new_and_lazy_closure(&self.lazy_cex));
                None
            }
        }
    }
}

impl FrAig {
    pub fn nword(&self) -> usize {
        self.simulation.nword()
    }

    pub fn cleanup_redundant(&mut self, node_map: &[Option<AigNodeId>]) {
        self.simulation.cleanup_redundant(node_map);
        let mut should_remove = Vec::new();
        for (k, cans) in &mut self.sim_map {
            let old = take(cans);
            for mut o in old {
                if let Some(dst) = node_map[o.node_id()] {
                    o.set_nodeid(dst);
                    cans.push(o);
                }
            }
            if cans.is_empty() {
                should_remove.push(*k);
            }
        }
        for should in should_remove {
            assert!(self.sim_map.remove(&should).is_some());
        }
        let old = take(&mut self.lazy_cex);
        for (id, old_sim) in old.into_iter().enumerate() {
            if let Some(dst) = node_map[id] {
                assert_eq!(dst, self.lazy_cex.len());
                self.lazy_cex.push(old_sim);
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
            let edge = AigEdge::new(idx, simulation[idx].compl());
            match candidate_map.get_mut(&simulation[idx].abs_hash_value()) {
                Some(candidate) => candidate.push(edge),
                None => {
                    assert!(candidate_map
                        .insert(simulation[idx].abs_hash_value(), vec![edge],)
                        .is_none());
                }
            }
        }
        candidate_map
    }

    pub fn fraig(&mut self, flag: bool) {
        assert!(self.fraig.is_none());
        let mut simulation = self.new_simulation(1);
        loop {
            let candidates = self.get_candidate(&simulation);
            // dbg!(candidates.keys().count());
            let mut update = false;
            let mut patterns = Vec::new();
            for candidate in candidates.values() {
                if candidate.len() == 1 {
                    continue;
                }
                for c in &candidate[1..] {
                    if let Some(s) = self.sat_solver.equivalence_check(candidate[0], *c) {
                        patterns.push(Self::gen_pattern(&self.nodes, s));
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
                        if flag {
                            unsafe { TOTAL_BUG += 1 };
                        }
                        assert_eq!(*k, simulation.abs_hash_value(*c).0);
                        self.merge_fe_node(*c, candidate[0]);
                    }
                }
                self.fraig = Some(FrAig {
                    simulation,
                    sim_map,
                    lazy_cex: Vec::new(),
                    ncex: 0,
                });
                self.fraig.as_mut().unwrap().lazy_cex =
                    self.fraig.as_ref().unwrap().default_lazy_cexs();
                dbg!(self.fraig.as_ref().unwrap().nword());
                return;
            } else {
                assert!(self.num_nodes() == patterns[0].len());
                let mut words = vec![0; self.num_nodes()];
                for (bit, pattern) in patterns.into_iter().enumerate() {
                    if bit > 0 && bit % (SimulationWord::BITS as usize) == 0 {
                        let submit = replace(&mut words, vec![0; self.num_nodes()]);
                        simulation.add_words(submit);
                    }
                    for (idx, p) in pattern.into_iter().enumerate() {
                        if p {
                            words[idx] |= 1 << bit;
                        }
                    }
                }
                simulation.add_words(words);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Aig;

    #[test]
    fn test1() {
        let aig = Aig::from_file("aigs/cec1.aag").unwrap();
        // aig.fraig();
        assert_eq!(aig.fraig.unwrap().sim_map.keys().len(), 6);
    }

    #[test]
    fn test2() {
        let aig = Aig::from_file("aigs/cec2.aag").unwrap();
        // aig.fraig();
        assert_eq!(aig.fraig.unwrap().sim_map.keys().len(), 8);
    }
}
