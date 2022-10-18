use std::ops::{BitAnd, Not};

use crate::Aig;
use rand::{rngs::ThreadRng, thread_rng, Rng};

type SimulationWord = usize;

#[derive(Clone, Debug)]
pub struct SimulationWords {
    words: Vec<SimulationWord>,
}

impl SimulationWords {
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
pub struct AigSimulation {
    nwords: usize,
    simulations: Vec<SimulationWords>,
}

impl AigSimulation {
    // pub fn new(aig: &Aig, nwords: usize) -> Self {
    //     let mut rwg = RandomWordGenerator::new();
    //     let mut simulations = Vec::new();
    //     for _ in 0..aig.num_inputs() {
    //         simulations.push(SimulationWords::new(nwords, &mut rwg));
    //     }
    //     for and in aig.ands_iter() {
    //         dbg!(and.node_id());
    //         let fanin0 = and.fanin0();
    //         let fanin1 = and.fanin1();
    //         assert!(simulations.len() == and.node_id());
    //         let sim0 = if fanin0.compl() {
    //             !simulations[fanin0.node_id()].clone()
    //         } else {
    //             simulations[fanin0.node_id()].clone()
    //         };
    //         let sim1 = if fanin1.compl() {
    //             !simulations[fanin1.node_id()].clone()
    //         } else {
    //             simulations[fanin1.node_id()].clone()
    //         };
    //         simulations.push(sim0 & sim1);
    //     }
    //     Self {
    //         nwords,
    //         simulations,
    //     }
    // }

    pub fn simulations(&self) -> &Vec<SimulationWords> {
        &self.simulations
    }
}

#[cfg(test)]
mod tests {
    use crate::Aig;

    use super::AigSimulation;
    #[test]
    fn test_simulation() {
        let aig = Aig::from_file("aigs/counter.aag").unwrap();
        dbg!(&aig);
        // let sim = AigSimulation::new(&aig, 4);
        // dbg!(&sim);
    }
}
