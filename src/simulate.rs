use crate::{Aig, AigEdge};
use rand::{rngs::ThreadRng, thread_rng, Rng};
use std::{
    fmt::{Display, Formatter, Result},
    ops::Index,
};

type SimulationWord = u16;

pub type SimulationWordsHash = u128;

const HASH_MUL: SimulationWordsHash = 1000000007;

#[derive(Clone, Debug)]
pub struct SimulationWords {
    hash: SimulationWordsHash,
    nbit_remain: usize,
    words: Vec<SimulationWord>,
}

impl SimulationWords {
    fn last_word_value(word: SimulationWord, nbit_remain: usize) -> SimulationWord {
        word & (SimulationWord::MAX >> nbit_remain)
    }

    fn get_bit_value(&self, index: usize) -> bool {
        let nword = index / SimulationWord::BITS as usize;
        let nbit = index % SimulationWord::BITS as usize;
        self.words[nword] & (1 << nbit) > 0
    }

    fn _set_bit_value(&mut self, index: usize, value: bool) {
        let nword = index / SimulationWord::BITS as usize;
        let nbit = index % SimulationWord::BITS as usize;
        if value {
            self.words[nword] |= 1 << nbit
        } else {
            self.words[nword] &= !(1 << nbit)
        }
    }

    fn calculate_hash(&mut self) {
        self.hash = 0;
        let compl = self.get_bit_value(0);
        for id in 0..self.words.len() - 1 {
            self.hash = unsafe {
                let word = if compl {
                    !self.words[id]
                } else {
                    self.words[id]
                };
                self.hash
                    .unchecked_mul(HASH_MUL as SimulationWordsHash)
                    .unchecked_add(word as SimulationWordsHash)
            };
        }
        let last = Self::last_word_value(
            if compl {
                !self.words.last().unwrap()
            } else {
                *self.words.last().unwrap()
            },
            self.nbit_remain,
        );

        self.hash = unsafe {
            self.hash
                .unchecked_mul(HASH_MUL as SimulationWordsHash)
                .unchecked_add(last as SimulationWordsHash)
        }
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
            words.push(Self::last_word_value(SimulationWord::MAX, nbit_remain));
        }
        let mut ret = Self {
            words,
            hash: 0,
            nbit_remain,
        };
        ret.calculate_hash();
        ret
    }
}

impl SimulationWords {
    pub fn nbit(&self) -> usize {
        self.words.len() * SimulationWord::BITS as usize - self.nbit_remain
    }

    pub fn abs_hash_value(&self) -> SimulationWordsHash {
        self.hash
    }

    pub fn compl(&self) -> bool {
        self.get_bit_value(0)
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
            words.push(Self::last_word_value(gen.rand_word(), nbit_remain));
        }
        let mut ret = Self {
            words,
            hash: 0,
            nbit_remain,
        };
        ret.calculate_hash();
        ret
    }

    fn push_bit(&mut self, bit: bool) {
        if self.nbit_remain == 0 {
            self.hash = unsafe {
                self.hash
                    .unchecked_mul(HASH_MUL as SimulationWordsHash)
                    .unchecked_add((bit ^ self.compl()) as SimulationWordsHash)
            };
            self.words.push(bit as SimulationWord);
            self.nbit_remain = SimulationWord::BITS as usize - 1;
        } else {
            let compl = self.compl();
            let last = self.words.pop().unwrap();
            let last_hash = if compl {
                Self::last_word_value(!last, self.nbit_remain)
            } else {
                last
            };
            self.hash = unsafe { self.hash.unchecked_sub(last_hash as SimulationWordsHash) };
            let last = last
                | ((bit as SimulationWord) << (SimulationWord::BITS as usize - self.nbit_remain));
            self.nbit_remain -= 1;
            let last_hash = if compl {
                Self::last_word_value(!last, self.nbit_remain)
            } else {
                last
            };
            self.hash = unsafe { self.hash.unchecked_add(last_hash as SimulationWordsHash) };
            self.words.push(last);
        }
    }
}

impl Display for SimulationWords {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        for word in self.words.iter().rev() {
            write!(f, "{:0>16b}", *word)?;
        }
        Ok(())
    }
}

// impl Not for SimulationWords {
//     type Output = SimulationWords;

//     fn not(self) -> Self::Output {
//         todo!()
//         // let mut words = Vec::new();
//         // for word in self.words {
//         //     words.push(!word);
//         // }
//         // if self.nbit_remain > 0 {
//         //     let last = words.pop().unwrap() & (SimulationWord::MAX >> self.nbit_remain);
//         //     words.push(last);
//         // }
//         // let hash = SimulationWords::calculate_hash(&words);
//         // Self {
//         //     words,
//         //     hash,
//         //     nbit_remain: self.nbit_remain,
//         // }
//     }
// }

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

    pub fn sim_and(&self, x: AigEdge, y: AigEdge) -> SimulationWords {
        let xwords = &self.simulations[x.node_id()];
        let ywords = &self.simulations[y.node_id()];
        assert!(xwords.nbit() == ywords.nbit());
        assert!(xwords.nbit_remain == ywords.nbit_remain);
        let mut words = Vec::new();
        for idx in 0..xwords.words.len() - 1 {
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
        let lastx = xwords.words.last().unwrap();
        let lasty = ywords.words.last().unwrap();
        let lastx = if x.compl() {
            SimulationWords::last_word_value(!lastx, xwords.nbit_remain)
        } else {
            *lastx
        };
        let lasty = if y.compl() {
            SimulationWords::last_word_value(!lasty, ywords.nbit_remain)
        } else {
            *lasty
        };
        words.push(lastx & lasty);
        let mut ret = SimulationWords {
            hash: 0,
            nbit_remain: xwords.nbit_remain,
            words,
        };
        ret.calculate_hash();
        ret
    }

    pub fn abs_hash_value(&self, e: AigEdge) -> (SimulationWordsHash, bool) {
        (
            self.simulations[e.node_id()].abs_hash_value(),
            e.compl() ^ self.simulations[e.node_id()].compl(),
        )
    }

    pub fn add_pattern(&mut self, pattern: Vec<bool>) {
        assert_eq!(pattern.len(), self.simulations.len());
        assert!(pattern[0]);
        for (i, p) in pattern.iter().enumerate().take(self.simulations.len()) {
            self.simulations[i].push_bit(*p)
        }
    }

    pub fn add_node(&mut self, sim: SimulationWords) {
        self.simulations.push(sim)
    }
}

impl Index<usize> for Simulation {
    type Output = SimulationWords;

    fn index(&self, index: usize) -> &Self::Output {
        &self.simulations[index]
    }
}

impl Aig {
    pub fn new_simulation(&self, nbits: usize) -> Simulation {
        let mut simulations = Simulation {
            simulations: vec![SimulationWords::true_word(nbits)],
        };
        for node in &self.nodes[1..] {
            if node.is_and() {
                let sim_and = simulations.sim_and(node.fanin0(), node.fanin1());
                simulations.simulations.push(sim_and);
            } else {
                simulations.simulations.push(SimulationWords::new(nbits));
            }
        }
        simulations
    }
}

#[cfg(test)]
mod tests {
    use super::SimulationWords;
    use crate::{Aig, AigEdge};

    #[test]
    fn test_words() {
        let mut words = SimulationWords::new(16);
        println!("{}", words);
        words.push_bit(true);
        println!("{}", words);
        words.push_bit(false);
        println!("{}", words);
        words.push_bit(true);
        println!("{}", words);
    }

    #[test]
    fn test_simulation() {
        let aig = Aig::from_file("aigs/counter-2bit.aag").unwrap();
        println!("{}", aig);
        let sim = aig.new_simulation(126);
        for s in &sim.simulations {
            println!("{:} {}", s, s.abs_hash_value());
        }
    }
}
