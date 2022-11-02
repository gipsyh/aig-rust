use crate::{Aig, AigEdge, AigNodeId};
use rand::{rngs::ThreadRng, thread_rng, Rng};
use std::{
    fmt::{Display, Formatter, Result},
    iter::repeat,
    mem::take,
    ops::Index,
    simd::Simd,
};

pub type SimulationWord = u64;

pub type SimdSimulationWord = Simd<SimulationWord, 64>;

pub const SIMULATION_TRUE_WORD: SimulationWord = SimulationWord::MAX;

pub type SimulationWordsHash = SimulationWord;

// const HASH_MUL: SimulationWordsHash = 4294967311;

// fn hash_function(hash: &mut SimulationWordsHash, word: SimulationWord) {
//     *hash = unsafe {
//         hash.unchecked_mul(HASH_MUL)
//             .unchecked_add(word as SimulationWordsHash)
//     }
// }

#[inline]
fn hash_function(hash: &mut SimulationWordsHash, mut word: SimulationWord) {
    word = ((word >> 16) ^ word) * 0x45d9f3b;
    word = ((word >> 16) ^ word) * 0x45d9f3b;
    word = (word >> 16) ^ word;
    *hash = *hash ^ (word + 0x9e3779b9 + (*hash << 6) + (*hash >> 2));
}

#[inline]
fn simd_hash_function(hash: &mut SimulationWordsHash, word: &SimdSimulationWord) {
    static SIMD_16: SimdSimulationWord = Simd::from_array([16; SimdSimulationWord::LANES]);
    static SIMD_MUL: SimdSimulationWord = Simd::from_array([0x45d9f3b; SimdSimulationWord::LANES]);
    static SIMD_ADD: SimdSimulationWord = Simd::from_array([0x9e3779b9; SimdSimulationWord::LANES]);
    let mut simd_word = ((word >> SIMD_16) ^ word) * SIMD_MUL;
    simd_word = ((simd_word >> SIMD_16) ^ simd_word) * SIMD_MUL;
    simd_word = ((simd_word >> SIMD_16) ^ simd_word) + SIMD_ADD;
    for w in simd_word.as_array() {
        *hash = *hash ^ (*w + (*hash << 6) + (*hash >> 2));
    }
}

#[derive(Clone, Debug)]
pub struct SimulationWords {
    hash: SimulationWordsHash,
    simd_words: Vec<SimdSimulationWord>,
    remain_words: SimdSimulationWord,
    nword_remain: usize,
    compl: bool,
}

impl SimulationWords {
    #[inline]
    fn calculate_hash(&mut self) {
        self.hash = 0;
        self.compl = self.simd_words[0][0] & 1 > 0;
        for simd_word in self.simd_words.iter() {
            for word in simd_word.as_array() {
                hash_function(&mut self.hash, if self.compl { !word } else { *word });
            }
        }
        for word in &self.remain_words[0..self.nword_remain] {
            hash_function(&mut self.hash, if self.compl { !word } else { *word });
        }
    }

    fn new_with_simd_words(
        simd_words: Vec<SimdSimulationWord>,
        remain_words_vec: Vec<SimulationWord>,
    ) -> Self {
        let mut remain_words = SimdSimulationWord::default();
        for (src, dst) in remain_words_vec.iter().zip(remain_words.as_mut_array()) {
            *dst = *src;
        }
        let mut ret = SimulationWords {
            hash: 0,
            compl: false,
            simd_words,
            remain_words,
            nword_remain: remain_words_vec.len(),
        };
        ret.calculate_hash();
        ret
    }

    fn true_words(nword: usize) -> Self {
        let nsimd = nword / SimdSimulationWord::LANES;
        let nremain = nword % SimdSimulationWord::LANES;
        let simd_words = repeat(())
            .take(nsimd)
            .map(|_| SimdSimulationWord::from([SIMULATION_TRUE_WORD; SimdSimulationWord::LANES]))
            .collect();
        let remain_words = repeat(())
            .take(nremain)
            .map(|_| SIMULATION_TRUE_WORD)
            .collect();
        SimulationWords::new_with_simd_words(simd_words, remain_words)
    }
}

impl SimulationWords {
    pub fn nword(&self) -> usize {
        self.simd_words.len() * SimdSimulationWord::LANES + self.nword_remain
    }

    pub fn abs_hash_value(&self) -> SimulationWordsHash {
        self.hash
    }

    pub fn compl(&self) -> bool {
        self.compl
    }

    pub fn new(nword: usize) -> Self {
        let mut gen = RandomWordGenerator::new();
        let nsimd = nword / SimdSimulationWord::LANES;
        let nremain = nword % SimdSimulationWord::LANES;
        let simd_words = repeat(())
            .take(nsimd)
            .map(|_| gen.rand_simd_word())
            .collect();
        let remain_words = repeat(()).take(nremain).map(|_| gen.rand_word()).collect();
        SimulationWords::new_with_simd_words(simd_words, remain_words)
    }

    fn push_word(&mut self, word: SimulationWord) {
        hash_function(&mut self.hash, if self.compl { !word } else { word });
        self.remain_words[self.nword_remain] = word;
        self.nword_remain += 1;
        if self.nword_remain == SimdSimulationWord::LANES {
            self.nword_remain = 0;
            self.simd_words.push(take(&mut self.remain_words));
        }
    }

    fn push_simd_word(&mut self, word: SimdSimulationWord) {
        assert!(self.nword_remain == 0);
        simd_hash_function(&mut self.hash, &if self.compl { !word } else { word });
        self.simd_words.push(word);
    }
}

impl Display for SimulationWords {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        for simd_word in self.simd_words.iter().rev() {
            for word in simd_word.as_array() {
                write!(f, "{:0>64b}", *word)?;
            }
        }
        for word in self.remain_words.as_array().iter().take(self.nword_remain) {
            write!(f, "{:0>64b}", *word)?;
        }
        Ok(())
    }
}

impl Index<usize> for SimulationWords {
    type Output = SimulationWord;

    fn index(&self, index: usize) -> &Self::Output {
        let nsimd = index / SimdSimulationWord::LANES;
        let nremain = index % SimdSimulationWord::LANES;
        if nsimd < self.simd_words.len() {
            &self.simd_words[nsimd][nremain]
        } else {
            &self.remain_words[nremain]
        }
    }
}

pub struct RandomWordGenerator {
    rng: ThreadRng,
}

impl RandomWordGenerator {
    pub fn new() -> Self {
        Self { rng: thread_rng() }
    }

    pub fn rand_word(&mut self) -> SimulationWord {
        self.rng.gen()
    }

    pub fn rand_simd_word(&mut self) -> SimdSimulationWord {
        let mut ret = SimdSimulationWord::default();
        for word in ret.as_mut_array() {
            *word = self.rng.gen()
        }
        ret
    }
}

#[derive(Debug, Clone)]
pub struct Simulation {
    simulations: Vec<SimulationWords>,
}

impl Simulation {
    pub fn num_nodes(&self) -> usize {
        self.simulations.len()
    }

    pub fn nword(&self) -> usize {
        self.simulations[0].nword()
    }

    #[inline]
    pub fn sim_and(&self, x: AigEdge, y: AigEdge) -> SimulationWords {
        let xwords = &self.simulations[x.node_id()];
        let ywords = &self.simulations[y.node_id()];
        let mut simd_words = Vec::with_capacity(xwords.simd_words.capacity());
        let mut remain_words = SimdSimulationWord::default();
        let simd_words_remain = simd_words.spare_capacity_mut();
        let edge_word = |word: &SimulationWord, edge: AigEdge| {
            if edge.compl() {
                !*word
            } else {
                *word
            }
        };
        let compl =
            (edge_word(&xwords.simd_words[0][0], x) & edge_word(&ywords.simd_words[0][0], y) & 1)
                > 0;
        let mut hash = 0;
        for (idx, simd_word_remain) in simd_words_remain
            .iter_mut()
            .enumerate()
            .take(xwords.simd_words.len())
        {
            let simd_word = match (x.compl(), y.compl()) {
                (true, true) => (!xwords.simd_words[idx]) & (!ywords.simd_words[idx]),
                (true, false) => (!xwords.simd_words[idx]) & (ywords.simd_words[idx]),
                (false, true) => (xwords.simd_words[idx]) & (!ywords.simd_words[idx]),
                (false, false) => xwords.simd_words[idx] & ywords.simd_words[idx],
            };
            for word in simd_word.as_array() {
                hash_function(&mut hash, if compl { !word } else { *word });
            }
            simd_word_remain.write(simd_word);
        }
        for (idx, remain_word) in remain_words
            .as_mut_array()
            .iter_mut()
            .enumerate()
            .take(xwords.nword_remain)
        {
            let word =
                edge_word(&xwords.remain_words[idx], x) & edge_word(&ywords.remain_words[idx], y);
            hash_function(&mut hash, if compl { !word } else { word });
            *remain_word = word;
        }
        unsafe { simd_words.set_len(xwords.simd_words.len()) };
        SimulationWords {
            hash,
            simd_words,
            remain_words,
            compl,
            nword_remain: xwords.nword_remain,
        }
    }

    pub fn abs_hash_value(&self, e: AigEdge) -> (SimulationWordsHash, bool) {
        (
            self.simulations[e.node_id()].abs_hash_value(),
            e.compl() ^ self.simulations[e.node_id()].compl(),
        )
    }

    pub fn add_words(&mut self, pattern: Vec<SimulationWord>) {
        assert_eq!(pattern.len(), self.simulations.len());
        for (i, p) in pattern.iter().enumerate() {
            self.simulations[i].push_word(*p);
        }
    }

    pub fn add_simd_word(&mut self, simd_word: Vec<SimdSimulationWord>) {
        assert_eq!(simd_word.len(), self.simulations.len());
        for (i, p) in simd_word.iter().enumerate() {
            self.simulations[i].push_simd_word(*p);
        }
    }

    pub fn add_node(&mut self, sim: SimulationWords) {
        assert!(sim.nword() == self.nword());
        self.simulations.push(sim)
    }
}

impl Simulation {
    pub fn cleanup_redundant(&mut self, node_map: &[Option<AigNodeId>]) {
        let old = take(&mut self.simulations);
        for (id, old_sim) in old.into_iter().enumerate() {
            if let Some(dst) = node_map[id] {
                assert_eq!(dst, self.simulations.len());
                self.simulations.push(old_sim);
            }
        }
    }
}

impl Index<usize> for Simulation {
    type Output = SimulationWords;

    fn index(&self, index: usize) -> &Self::Output {
        &self.simulations[index]
    }
}

impl Aig {
    pub fn new_simulation(&self, nsimd_word: usize) -> Simulation {
        let nwords = nsimd_word * SimdSimulationWord::LANES;
        let mut simulations = Simulation {
            simulations: vec![SimulationWords::true_words(nwords)],
        };
        for node in &self.nodes[1..] {
            if node.is_and() {
                let sim_and = simulations.sim_and(node.fanin0(), node.fanin1());
                simulations.simulations.push(sim_and);
            } else {
                simulations.simulations.push(SimulationWords::new(nwords));
            }
        }
        simulations
    }
}

#[cfg(test)]
mod tests {
    use crate::Aig;

    #[test]
    fn test_simulation() {
        let aig = Aig::from_file("aigs/counter-2bit.aag").unwrap();
        println!("{}", aig);
        let sim = aig.new_simulation(2);
        for s in &sim.simulations {
            println!("{:} {}", s, s.abs_hash_value());
        }
    }
}
