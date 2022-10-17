use std::{
    io,
    ops::{Index, Range},
    path::Path,
    slice::Iter,
};

mod fraig;
mod simulate;

type AigNodeId = usize;

#[derive(Debug)]
pub struct AigNode {
    id: AigNodeId,
    fanin0: Option<AigEdge>,
    fanin1: Option<AigEdge>,
}

impl AigNode {
    pub fn new_input(id: usize) -> Self {
        Self {
            id,
            fanin0: None,
            fanin1: None,
        }
    }

    pub fn new_and(id: usize, fanin0: AigEdge, fanin1: AigEdge) -> Self {
        Self {
            id,
            fanin0: Some(fanin0),
            fanin1: Some(fanin1),
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

    pub fn fanin0(&self) -> Option<AigEdge> {
        self.fanin0
    }

    pub fn fanin1(&self) -> Option<AigEdge> {
        self.fanin1
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AigEdge {
    /// if id is none, it means the node is true
    id: Option<AigNodeId>,
    complement: bool,
}

impl AigEdge {
    pub fn new(id: AigNodeId, complement: bool) -> Self {
        Self {
            id: Some(id),
            complement,
        }
    }

    pub fn value(&self) -> Option<bool> {
        match self.id {
            Some(_) => None,
            None => Some(!self.complement),
        }
    }

    pub fn node_id(&self) -> AigNodeId {
        self.id.unwrap()
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
}

impl Aig {
    pub fn new(
        nodes: Vec<AigNode>,
        latchs: Vec<AigLatch>,
        outputs: Vec<AigEdge>,
        inputs: Range<usize>,
        ands: Range<usize>,
    ) -> Self {
        Self {
            nodes,
            latchs,
            outputs,
            inputs,
            ands,
        }
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

    pub fn top_sort(&mut self) {}

    fn num_inputs(&self) -> usize {
        self.inputs.len()
    }

    fn num_ands(&self) -> usize {
        self.ands.len()
    }

    pub fn num_nodes(&self) -> usize {
        self.nodes.len()
    }

    pub fn inputs_iter(&self) -> Iter<AigNode> {
        todo!()
    }

    pub fn ands_iter(&self) -> Iter<AigNode> {
        self.nodes[self.ands.clone()].iter()
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
