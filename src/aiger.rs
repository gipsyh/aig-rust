use crate::{Aig, AigEdge, AigLatch, AigNode};
use std::{collections::HashMap, io, mem::take, path::Path};

impl Aig {
    fn setup_levels(&mut self) {
        let mut levels = vec![0; self.num_nodes()];
        for and in self.ands_iter() {
            let fanin0 = and.fanin0().node_id();
            let fanin1 = and.fanin1().node_id();
            levels[and.node_id()] = levels[fanin0].max(levels[fanin1]) + 1;
        }
        for (id, node) in levels.iter().enumerate().take(self.num_nodes()) {
            self.nodes[id].level = *node;
        }
    }

    fn setup_fanouts(&mut self) {
        let mut fanouts = vec![vec![]; self.num_nodes()];
        for and in self.ands_iter() {
            let fanin0 = and.fanin0();
            let fanin0id = fanin0.node_id();
            let compl = fanin0.compl();
            fanouts[fanin0id].push(AigEdge::new(and.id, compl));
            let fanin1 = and.fanin1();
            let fanin1id = fanin1.node_id();
            let compl = fanin1.compl();
            fanouts[fanin1id].push(AigEdge::new(and.id, compl));
        }
        for (id, node) in fanouts.iter_mut().enumerate().take(self.num_nodes()) {
            self.nodes[id].fanouts = take(node);
        }
    }

    // fn setup_subnode_size(&mut self) {
    //     for ci in 0..self.nodes.len() {
    //         self.nodes[ci].size += 1;
    //         let mut flag = vec![false; self.num_nodes()];
    //         let mut queue = VecDeque::new();
    //         for fanout in &self.nodes[ci].fanouts {
    //             if !flag[fanout.id] {
    //                 queue.push_back(fanout.id);
    //                 flag[fanout.id] = true;
    //             }
    //         }
    //         while !queue.is_empty() {
    //             let node = queue.pop_front().unwrap();
    //             self.nodes[node].size += 1;
    //             for fanout in &self.nodes[node].fanouts {
    //                 if !flag[fanout.id] {
    //                     queue.push_back(fanout.id);
    //                     flag[fanout.id] = true;
    //                 }
    //             }
    //         }
    //     }
    // }

    pub fn from_file<P: AsRef<Path>>(file: P) -> io::Result<Self> {
        let file = std::fs::File::open(file)?;
        let aiger = aiger::Reader::from_reader(file).unwrap();
        let header = aiger.header();
        let mut nodes: Vec<AigNode> = Vec::with_capacity(header.i + header.l + header.a + 1);
        let nodes_remaining = nodes.spare_capacity_mut();
        nodes_remaining[0].write(AigNode::new_true(0));
        let mut outputs = Vec::new();
        let mut bads = Vec::new();
        let mut cinputs = Vec::new();
        let mut latchs = Vec::new();
        for obj in aiger.records() {
            let obj = obj.unwrap();
            match obj {
                aiger::Aiger::Input(input) => {
                    let id = input.0 / 2;
                    nodes_remaining[id].write(AigNode::new_prime_input(id));
                    cinputs.push(id);
                }
                aiger::Aiger::Latch {
                    output,
                    input,
                    init,
                } => {
                    let id = output.0 / 2;
                    nodes_remaining[id].write(AigNode::new_latch_input(id));
                    latchs.push(AigLatch::new(
                        id,
                        AigEdge::new(input.0 / 2, input.0 & 0x1 != 0),
                        init,
                    ));
                    cinputs.push(id);
                }
                aiger::Aiger::Output(o) => outputs.push(AigEdge::new(o.0 / 2, o.0 & 0x1 != 0)),
                aiger::Aiger::BadState(b) => bads.push(AigEdge::new(b.0 / 2, b.0 & 0x1 != 0)),
                aiger::Aiger::AndGate { output, inputs } => {
                    let id = output.0 / 2;
                    nodes_remaining[id].write(AigNode::new_and(
                        id,
                        AigEdge::new(inputs[0].0 / 2, inputs[0].0 & 0x1 != 0),
                        AigEdge::new(inputs[1].0 / 2, inputs[1].0 & 0x1 != 0),
                        0,
                    ));
                }
                aiger::Aiger::Symbol {
                    type_spec: _,
                    position: _,
                    symbol: _,
                } => (),
            }
        }
        unsafe { nodes.set_len(header.i + header.l + header.a + 1) };
        let mut ret = Self {
            nodes,
            cinputs,
            latchs,
            outputs,
            bads,
            num_inputs: header.i,
            num_latchs: header.l,
            num_ands: header.a,
            strash_map: HashMap::new(),
        };
        ret.setup_levels();
        ret.setup_fanouts();
        ret.setup_strash();
        Ok(ret)
    }
}

#[cfg(test)]
mod tests {
    use crate::Aig;

    #[test]
    fn test() {
        let aig = Aig::from_file("aigs/counter-3bit.aag").unwrap();
        println!("{}", aig);
    }
}
