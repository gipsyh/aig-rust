use std::{
    collections::{HashMap, VecDeque},
    io,
    mem::swap,
    ops::{Index, Range},
    path::Path,
    slice::{Iter, IterMut},
};

mod fraig;
mod simulate;
mod strash;

type AigNodeId = usize;

#[derive(Debug)]
pub struct AigNode {
    id: AigNodeId,
    size: usize,
    fanin0: Option<AigEdge>,
    fanin1: Option<AigEdge>,
    fanouts: Vec<AigEdge>,
}

impl AigNode {
    pub fn new_input(id: usize) -> Self {
        Self {
            id,
            size: 0,
            fanin0: None,
            fanin1: None,
            fanouts: Vec::new(),
        }
    }

    pub fn new_and(id: usize, mut fanin0: AigEdge, mut fanin1: AigEdge) -> Self {
        if fanin0.node_id() > fanin1.node_id() {
            swap(&mut fanin0, &mut fanin1);
        }
        Self {
            id,
            size: 0,
            fanin0: Some(fanin0),
            fanin1: Some(fanin1),
            fanouts: Vec::new(),
        }
    }

    pub fn node_id(&self) -> AigNodeId {
        self.id
    }

    pub fn is_and(&self) -> bool {
        self.fanin0.is_some() && self.fanin1.is_some()
    }

    pub fn is_input(&self) -> bool {
        self.fanin0.is_none() && self.fanin1.is_none()
    }

    pub fn fanin0(&self) -> AigEdge {
        self.fanin0.unwrap()
    }

    pub fn fanin1(&self) -> AigEdge {
        self.fanin1.unwrap()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AigEdge {
    /// if id is none, it means the node is true
    id: AigNodeId,
    complement: bool,
}

impl AigEdge {
    pub fn new(id: AigNodeId, complement: bool) -> Self {
        Self { id: id, complement }
    }

    pub fn node_id(&self) -> AigNodeId {
        self.id
    }

    pub fn compl(&self) -> bool {
        self.complement
    }
}

#[derive(Debug)]
pub struct AigLatch {
    input: AigNodeId,
    next: AigEdge,
}

impl AigLatch {
    pub fn new(input: AigNodeId, next: AigEdge) -> Self {
        Self { input, next }
    }
}

#[derive(Debug)]
pub struct Aig {
    nodes: Vec<AigNode>,
    latchs: Vec<AigLatch>,
    outputs: Vec<AigEdge>,
    inputs: Range<usize>,
    ands: Range<usize>,
    strash_map: Option<HashMap<(AigNodeId, bool, AigNodeId, bool), AigNodeId>>,
}

impl Aig {
    fn new(
        nodes: Vec<AigNode>,
        latchs: Vec<AigLatch>,
        outputs: Vec<AigEdge>,
        inputs: Range<usize>,
        ands: Range<usize>,
    ) -> Self {
        let mut ret = Self {
            nodes,
            latchs,
            outputs,
            inputs,
            ands,
            strash_map: None,
        };
        ret.setup_fanouts();
        ret.setup_subnode_size();
        ret
    }

    fn setup_fanouts(&mut self) {
        for and_idx in self.ands.clone() {
            let fanin0 = &self.nodes[and_idx].fanin0();
            let fanin0id = fanin0.node_id();
            let compl = fanin0.compl();
            let fanin0 = &mut self.nodes[fanin0id];
            fanin0.fanouts.push(AigEdge::new(and_idx, compl));
            let fanin1 = &self.nodes[and_idx].fanin1();
            let fanin1id = fanin1.node_id();
            let compl = fanin1.compl();
            let fanin1 = &mut self.nodes[fanin1id];
            fanin1.fanouts.push(AigEdge::new(and_idx, compl));
        }
    }

    fn setup_subnode_size(&mut self) {
        for ci in 0..self.nodes.len() {
            self.nodes[ci].size += 1;
            let mut flag = vec![false; self.num_nodes()];
            let mut queue = VecDeque::new();
            for fanout in &self.nodes[ci].fanouts {
                if !flag[fanout.id] {
                    queue.push_back(fanout.id);
                    flag[fanout.id] = true;
                }
            }
            while !queue.is_empty() {
                let node = queue.pop_front().unwrap();
                self.nodes[node].size += 1;
                for fanout in &self.nodes[node].fanouts {
                    if !flag[fanout.id] {
                        queue.push_back(fanout.id);
                        flag[fanout.id] = true;
                    }
                }
            }
        }

        // for idx in self.ands.clone() {
        //     dbg!(idx);
        //     let and = &self.nodes[idx];
        //     let fanin0 = self.nodes[and.fanin0().node_id()].size.unwrap();
        //     let fanin1 = self.nodes[and.fanin1().node_id()].size.unwrap();
        //     let and = &mut self.nodes[idx];
        //     and.size = Some(fanin0 + fanin1 + 1);
        // }
    }

    pub fn from_file<P: AsRef<Path>>(file: P) -> io::Result<Self> {
        let file = std::fs::File::open(file)?;
        let aiger = aiger::Reader::from_reader(file).unwrap();
        let header = aiger.header();
        let inputs = 0..header.i + header.l;
        let ands = header.i + header.l..header.i + header.l + header.a;
        let mut nodes: Vec<AigNode> = Vec::with_capacity(header.m);
        let nodes_remaining = nodes.spare_capacity_mut();
        let mut outputs = Vec::new();
        let mut latchs = Vec::new();
        for obj in aiger.records() {
            let obj = obj.unwrap();
            match obj {
                aiger::Aiger::Input(input) => {
                    let id = input.0 / 2 - 1;
                    nodes_remaining[id].write(AigNode::new_input(id));
                }
                aiger::Aiger::Latch { output, input } => {
                    let id = output.0 / 2 - 1;
                    nodes_remaining[id].write(AigNode::new_input(id));
                    latchs.push(AigLatch::new(
                        output.0 / 2 - 1,
                        AigEdge::new(input.0 / 2 - 1, input.0 & 0x1 != 0),
                    ))
                }
                aiger::Aiger::Output(o) => outputs.push(AigEdge::new(o.0 / 2 - 1, o.0 & 0x1 != 0)),
                aiger::Aiger::AndGate { output, inputs } => {
                    let id = output.0 / 2 - 1;
                    nodes_remaining[id].write(AigNode::new_and(
                        id,
                        AigEdge::new(inputs[0].0 / 2 - 1, inputs[0].0 & 0x1 != 0),
                        AigEdge::new(inputs[1].0 / 2 - 1, inputs[1].0 & 0x1 != 0),
                    ));
                }
                aiger::Aiger::Symbol {
                    type_spec,
                    position,
                    symbol,
                } => todo!(),
            }
        }
        unsafe { nodes.set_len(header.m) };
        Ok(Self::new(nodes, latchs, outputs, inputs, ands))
    }
}

impl Aig {
    fn num_inputs(&self) -> usize {
        self.inputs.len()
    }

    fn num_ands(&self) -> usize {
        self.ands.len()
    }

    pub fn num_nodes(&self) -> usize {
        self.nodes.len()
    }

    pub fn pinputs_iter(&self) -> Iter<AigNode> {
        todo!()
    }

    pub fn cinputs_iter(&self) -> Iter<AigNode> {
        self.nodes[self.inputs.clone()].iter()
    }

    pub fn cinputs_iter_mut(&mut self) -> IterMut<AigNode> {
        self.nodes[self.inputs.clone()].iter_mut()
    }

    pub fn ands_iter(&self) -> Iter<AigNode> {
        self.nodes[self.ands.clone()].iter()
    }

    pub fn ands_iter_mut(&mut self) -> IterMut<AigNode> {
        self.nodes[self.ands.clone()].iter_mut()
    }
}

impl Index<AigNodeId> for Aig {
    type Output = AigNode;

    fn index(&self, index: AigNodeId) -> &Self::Output {
        &self.nodes[index]
    }
}

#[cfg(test)]
mod tests {
    use crate::Aig;
    #[test]
    fn test_from_file() {
        let aig = Aig::from_file("aigs/counter.aag").unwrap();
        dbg!(aig);
    }
}
