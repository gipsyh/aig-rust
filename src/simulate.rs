use crate::{Aig, AigEdge};
use rand::{rngs::ThreadRng, thread_rng, Rng};
use std::{
    fmt::{Display, Formatter, Result},
    ops::{BitAnd, Not},
    vec,
};

type SimulationWord = usize;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct SimulationWords {
    words: Vec<SimulationWord>,
}

impl Display for SimulationWords {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{:0>64b}", self.words[0])
    }
}

impl SimulationWords {
    fn true_word(nwords: usize) -> Self {
        let mut words = Vec::new();
        for _ in 0..nwords {
            words.push(usize::MAX);
        }
        Self { words }
    }

    pub fn new(nwords: usize) -> Self {
        let mut gen = RandomWordGenerator::new();
        let mut words = Vec::new();
        for _ in 0..nwords {
            words.push(gen.rand_word());
        }
        Self { words }
    }

    fn push(&mut self, word: SimulationWord) {
        self.words.push(word)
    }

    fn append(&mut self, other: &mut Self) {
        self.words.append(&mut other.words);
    }
}

impl BitAnd for SimulationWords {
    type Output = SimulationWords;

    fn bitand(self, rhs: Self) -> Self::Output {
        assert!(self.words.len() == rhs.words.len());
        let mut words = Vec::new();
        for idx in 0..self.words.len() {
            words.push(self.words[idx] & rhs.words[idx]);
        }
        Self { words }
    }
}

impl Not for SimulationWords {
    type Output = SimulationWords;

    fn not(self) -> Self::Output {
        let mut words = Vec::new();
        for word in self.words {
            words.push(!word);
        }
        Self { words }
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
    nwords: usize,
    simulations: Vec<SimulationWords>,
}

impl Simulation {
    pub fn nwords(&self) -> usize {
        self.nwords
    }

    pub fn sim_value(&self, e: AigEdge) -> SimulationWords {
        if e.compl() {
            !self.simulations[e.node_id()].clone()
        } else {
            self.simulations[e.node_id()].clone()
        }
    }

    pub fn simulations(&self) -> &Vec<SimulationWords> {
        &self.simulations
    }

    fn merge(&mut self, other: &mut Self) {
        assert!(self.simulations.len() == other.simulations.len());
        for i in 0..self.simulations.len() {
            self.simulations[i].append(&mut other.simulations[i])
        }
        self.nwords += other.nwords
    }

    pub fn add_pattern(&mut self, pattern: Vec<bool>) {
        assert_eq!(pattern.len() + 1, self.simulations.len());
        self.nwords += 1;
        self.simulations[0].push(SimulationWord::MAX - 1);
        for i in 1..self.simulations.len() {
            if pattern[i - 1] {
                self.simulations[i].push(SimulationWord::MAX - 1)
            } else {
                self.simulations[i].push(0)
            }
        }
    }

    pub fn add_node(&mut self, sim: SimulationWords) {
        self.simulations.push(sim)
    }
}

impl Aig {
    pub fn new_simulation(&self, nwords: usize) -> Simulation {
        let mut simulations = vec![SimulationWords::true_word(nwords)];
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
                simulations.push(SimulationWords::new(nwords));
            }
        }
        Simulation {
            nwords,
            simulations,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Aig;

    #[test]
    fn test_simulation() {
        let aig = Aig::from_file("aigs/counter-2bit.aag").unwrap();
        println!("{}", aig);
        let sim = aig.new_simulation(1);
        for s in sim.simulations {
            println!("{:}", s);
        }
    }
}
