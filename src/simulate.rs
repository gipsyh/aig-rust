use crate::{Aig, AigEdge, AigNodeId};
use rand::{rngs::ThreadRng, thread_rng, Rng};
use std::{
    fmt::{Display, Formatter, Result},
    iter::repeat,
    mem::take,
    ops::Index,
};

pub type SimulationWord = u32;

pub const SIMULATION_TRUE_WORD: SimulationWord = SimulationWord::MAX;

pub type SimulationWordsHash = u64;

const HASH_MUL: SimulationWordsHash = 4294967311;

// const HASH_MUL_PRIMES: [SimulationWord; 16] = [
//     1291, 1699, 1999, 2357, 2953, 3313, 3907, 4177, 4831, 5147, 5647, 6343, 6899, 7103, 7873, 8147,
// ];

#[derive(Clone, Debug)]
pub struct SimulationWords {
    hash: SimulationWordsHash,
    words: Vec<SimulationWord>,
    compl: bool,
}

impl SimulationWords {
    #[inline]
    fn calculate_hash(&mut self) {
        self.hash = 0;
        self.compl = self.words[0] & 1 > 0;
        for id in 0..self.words.len() {
            // self.hash ^= unsafe {
            //     if self.compl {
            //         !self.words[id]
            //     } else {
            //         self.words[id]
            //     }
            //     .unchecked_mul(HASH_MUL_PRIMES[id & 0xf] as SimulationWord)
            // };
            self.hash = unsafe {
                self.hash
                    .unchecked_mul(HASH_MUL)
                    .unchecked_add(if self.compl {
                        !self.words[id]
                    } else {
                        self.words[id]
                    } as SimulationWordsHash)
            }
        }
    }

    fn true_words(nword: usize) -> Self {
        let mut ret = Self {
            words: repeat(SimulationWord::MAX).take(nword).collect(),
            hash: 0,
            compl: false,
        };
        ret.calculate_hash();
        ret
    }
}

impl SimulationWords {
    pub fn nword(&self) -> usize {
        self.words.len()
    }

    pub fn abs_hash_value(&self) -> SimulationWordsHash {
        self.hash
    }

    pub fn compl(&self) -> bool {
        self.compl
    }

    fn new_with_words(words: Vec<SimulationWord>) -> Self {
        let mut ret = SimulationWords {
            words,
            hash: 0,
            compl: false,
        };
        ret.calculate_hash();
        ret
    }

    pub fn new(nword: usize) -> Self {
        let mut gen = RandomWordGenerator::new();
        let words = repeat(()).take(nword).map(|_| gen.rand_word()).collect();
        SimulationWords::new_with_words(words)
    }

    fn push_word(&mut self, word: SimulationWord) {
        // self.hash ^= unsafe {
        //     if self.compl { !word } else { word }
        //         .unchecked_mul(HASH_MUL_PRIMES[self.words.len() & 0xf] as SimulationWord)
        // };
        self.hash = unsafe {
            self.hash
                .unchecked_mul(HASH_MUL)
                .unchecked_add(if self.compl { !word } else { word } as SimulationWordsHash)
        };
        self.words.push(word);
    }
}

impl Display for SimulationWords {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        for word in self.words.iter().rev() {
            write!(f, "{:0>32b}", *word)?;
        }
        Ok(())
    }
}

struct RandomWordGenerator {
    rng: ThreadRng,
}

impl RandomWordGenerator {
    fn new() -> Self {
        Self { rng: thread_rng() }
    }

    fn rand_word(&mut self) -> SimulationWord {
        self.rng.gen()
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
        let mut words = Vec::with_capacity(self.nword());
        for idx in 0..xwords.words.len() {
            let xword = if x.compl() {
                !xwords.words[idx]
            } else {
                xwords.words[idx]
            };
            let yword = if y.compl() {
                !ywords.words[idx]
            } else {
                ywords.words[idx]
            };
            words.push(xword & yword);
        }
        SimulationWords::new_with_words(words)
    }

    pub fn abs_hash_value(&self, e: AigEdge) -> (SimulationWordsHash, bool) {
        (
            self.simulations[e.node_id()].abs_hash_value(),
            e.compl() ^ self.simulations[e.node_id()].compl(),
        )
    }

    pub fn add_words(&mut self, pattern: Vec<SimulationWord>) {
        assert_eq!(pattern.len(), self.simulations.len());
        for (i, p) in pattern.iter().enumerate().take(self.simulations.len()) {
            self.simulations[i].push_word(*p);
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
    pub fn new_simulation(&self, nwords: usize) -> Simulation {
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
