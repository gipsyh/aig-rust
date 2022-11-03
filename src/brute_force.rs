use crate::Aig;
use std::collections::HashSet;

impl Aig {
    pub fn input_iter(num: usize) -> impl Iterator<Item = Vec<bool>> {
        let num_pow: usize = 1 << num;
        (0..num_pow).map(move |input| {
            let mut ret = Vec::new();
            for i in 0..num {
                ret.push(input & (1 << i) > 0)
            }
            ret
        })
    }

    pub fn get_latch_init(&self) -> Vec<bool> {
        self.latchs.iter().map(|l| l.init).collect()
    }

    pub fn get_value(&self, inputs: &[bool], latchs: &[bool]) -> Vec<bool> {
        let mut value = vec![false; self.num_nodes()];
        value[0] = false;
        for (i, l) in self.latchs.iter().enumerate() {
            value[l.input] = latchs[i];
        }
        for (i, input) in self.inputs.iter().enumerate() {
            value[*input] = inputs[i];
        }
        for i in self.nodes_range() {
            if self.nodes[i].is_and() {
                let fanin0 = self.nodes[i].fanin0();
                let fanin1 = self.nodes[i].fanin1();
                let fanin0v = value[fanin0.node_id()] ^ fanin0.compl();
                let fanin1v = value[fanin1.node_id()] ^ fanin1.compl();
                value[i] = fanin0v & fanin1v;
            }
        }
        self.latchs
            .iter()
            .map(|l| value[l.next.node_id()] ^ l.next.compl())
            .collect()
    }

    pub fn brute_force(&mut self) {
        let mut reach = HashSet::new();
        reach.insert(self.get_latch_init());
        let mut frontier = reach.clone();
        for deep in 1.. {
            dbg!(deep);
            let mut new_frontier = HashSet::new();
            for r in &frontier {
                for input in Self::input_iter(self.inputs.len()) {
                    new_frontier.insert(self.get_value(&input, r));
                }
            }
            frontier.clear();
            for s in &new_frontier {
                if !reach.contains(s) {
                    reach.insert(s.clone());
                    frontier.insert(s.clone());
                }
            }
            if frontier.is_empty() {
                dbg!(deep);
                return;
            }
        }
    }
}
