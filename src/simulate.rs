use crate::Aig;
use rand::{rngs::ThreadRng, thread_rng, Rng};
use std::{
    fmt::{Display, Formatter, Result},
    ops::{BitAnd, Not},
};

type SimulationWord = usize;

#[derive(Clone, Debug)]
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
    fn new(nwords: usize, gen: &mut RandomWordGenerator) -> Self {
        let mut words = Vec::new();
        for _ in 0..nwords {
            words.push(gen.rand_word());
        }
        Self { words }
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
    pub fn simulations(&self) -> &Vec<SimulationWords> {
        &self.simulations
    }
}

impl Aig {
    pub fn new_simulation(&self, nwords: usize) -> Simulation {
        let mut rwg = RandomWordGenerator::new();
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
                simulations.push(SimulationWords::new(nwords, &mut rwg));
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
