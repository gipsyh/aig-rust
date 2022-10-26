use crate::{Aig, AigEdge};
use rand::{rngs::ThreadRng, thread_rng, Rng};
use std::{
    fmt::{Display, Formatter, Result},
    ops::{BitAnd, Not},
    vec,
};

type SimulationWord = u128;

pub type SimulationWordsHash = SimulationWord;

const HASH_MUL: SimulationWordsHash = 131;

#[derive(Clone, Debug)]
pub struct SimulationWords {
    hash: SimulationWordsHash,
    nbit_remain: usize,
    words: Vec<SimulationWord>,
}

impl SimulationWords {
    fn calculate_hash(bits: &[SimulationWord]) -> SimulationWordsHash {
        let mut ret: SimulationWordsHash = 0;
        for word in bits {
            ret = unsafe {
                ret.unchecked_mul(HASH_MUL as SimulationWord)
                    .unchecked_add(*word)
            };
        }
        ret
    }

    fn true_word(nbits: usize) -> Self {
        let mut words = Vec::new();
        let nword = nbits / SimulationWord::BITS as usize;
        for _ in 0..nword {
            words.push(SimulationWord::MAX);
        }
        let mut nbit_remain =
            SimulationWord::BITS as usize - (nbits % SimulationWord::BITS as usize);
        if nbit_remain == SimulationWord::BITS as usize {
            nbit_remain = 0;
        } else {
            words.push(SimulationWord::MAX >> nbit_remain);
        }
        dbg!(nbit_remain);
        let hash = SimulationWords::calculate_hash(&words);
        Self {
            words,
            hash,
            nbit_remain,
        }
    }
}

impl SimulationWords {
    pub fn nbit(&self) -> usize {
        self.words.len() * SimulationWord::BITS as usize - self.nbit_remain
    }

    pub fn hash_value(&self) -> SimulationWordsHash {
        self.hash
    }

    pub fn new(nbits: usize) -> Self {
        let mut gen = RandomWordGenerator::new();
        let mut words = Vec::new();
        let nword = nbits / SimulationWord::BITS as usize;
        for _ in 0..nword {
            words.push(gen.rand_word());
        }
        let mut nbit_remain =
            SimulationWord::BITS as usize - (nbits % SimulationWord::BITS as usize);
        if nbit_remain == SimulationWord::BITS as usize {
            nbit_remain = 0;
        } else {
            words.push(gen.rand_word() >> nbit_remain);
        }
        let hash = SimulationWords::calculate_hash(&words);
        Self {
            words,
            hash,
            nbit_remain,
        }
    }

    fn push_bit(&mut self, bit: bool) {
        if self.nbit_remain == 0 {
            let word = bit as SimulationWord;
            self.hash = unsafe {
                self.hash
                    .unchecked_mul(HASH_MUL as SimulationWord)
                    .unchecked_add(word)
            };
            self.words.push(word);
            self.nbit_remain = SimulationWord::BITS as usize - 1;
        } else {
            let last = self.words.pop().unwrap();
            self.hash = unsafe { self.hash.unchecked_sub(last) };
            let last = (last << 1) + bit as SimulationWord;
            self.hash = unsafe { self.hash.unchecked_add(last) };
            self.words.push(last);
            self.nbit_remain -= 1;
        }
    }
}

impl Display for SimulationWords {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{:0>128b}", self.words[0])
    }
}

impl BitAnd for SimulationWords {
    type Output = SimulationWords;

    fn bitand(self, rhs: Self) -> Self::Output {
        assert!(self.words.len() == rhs.words.len());
        assert!(self.nbit_remain == rhs.nbit_remain);
        let mut words = Vec::new();
        for idx in 0..self.words.len() {
            words.push(self.words[idx] & rhs.words[idx]);
        }
        let hash = SimulationWords::calculate_hash(&words);
        Self {
            words,
            hash,
            nbit_remain: self.nbit_remain,
        }
    }
}

impl Not for SimulationWords {
    type Output = SimulationWords;

    fn not(self) -> Self::Output {
        let mut words = Vec::new();
        for word in self.words {
            words.push(!word);
        }
        if self.nbit_remain > 0 {
            let last = words.pop().unwrap() & (SimulationWord::MAX >> self.nbit_remain);
            words.push(last);
        }
        let hash = SimulationWords::calculate_hash(&words);
        Self {
            words,
            hash,
            nbit_remain: self.nbit_remain,
        }
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

#[derive(Debug)]
pub struct Simulation {
    simulations: Vec<SimulationWords>,
}

impl Simulation {
    pub fn num_nodes(&self) -> usize {
        self.simulations.len()
    }

    pub fn nbit(&self) -> usize {
        self.simulations[0].nbit()
    }

    pub fn sim_value(&self, e: AigEdge) -> SimulationWords {
        if e.compl() {
            !self.simulations[e.node_id()].clone()
        } else {
            self.simulations[e.node_id()].clone()
        }
    }

    pub fn hash_value(&self, e: AigEdge) -> SimulationWordsHash {
        if e.compl() {
            (!self.simulations[e.node_id()].clone()).hash_value()
        } else {
            self.simulations[e.node_id()].hash_value()
        }
    }

    pub fn simulations(&self) -> &Vec<SimulationWords> {
        &self.simulations
    }

    pub fn add_pattern(&mut self, pattern: Vec<bool>) {
        assert_eq!(pattern.len() + 1, self.simulations.len());
        self.simulations[0].push_bit(true);
        for i in 1..self.simulations.len() {
            self.simulations[i].push_bit(pattern[i - 1])
        }
    }

    pub fn add_node(&mut self, sim: SimulationWords) {
        self.simulations.push(sim)
    }
}

impl Aig {
    pub fn new_simulation(&self, nbits: usize) -> Simulation {
        let mut simulations = vec![SimulationWords::true_word(nbits)];
        for node in &self.nodes[1..] {
            if node.is_and() {
                let fanin0 = node.fanin0();
                let fanin1 = node.fanin1();
                let sim0 = if fanin0.compl() {
                    !simulations[fanin0.node_id()].clone()
                } else {
                    simulations[fanin0.node_id()].clone()
                };
                let sim1 = if fanin1.compl() {
                    !simulations[fanin1.node_id()].clone()
                } else {
                    simulations[fanin1.node_id()].clone()
                };
                simulations.push(sim0 & sim1);
            } else {
                simulations.push(SimulationWords::new(nbits));
            }
        }
        Simulation { simulations }
    }
}

#[cfg(test)]
mod tests {
    use crate::Aig;

    #[test]
    fn test_simulation() {
        let aig = Aig::from_file("aigs/counter-2bit.aag").unwrap();
        println!("{}", aig);
        let sim = aig.new_simulation(126);
        for s in sim.simulations {
            println!("{:}", s);
        }
    }
}
