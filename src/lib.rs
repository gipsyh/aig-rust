use std::{io, ops::Range, path::Path};

type AigNodeId = usize;

#[derive(Debug)]
pub struct AigNode {
    fanin0: Option<AigEdge>,
    fanin1: Option<AigEdge>,
}

impl AigNode {
    pub fn new_input() -> Self {
        Self {
            fanin0: None,
            fanin1: None,
        }
    }

    fn new_and(fanin0: AigEdge, fanin1: AigEdge) -> Self {
        Self {
            fanin0: Some(fanin0),
            fanin1: Some(fanin1),
        }
    }
}

#[derive(Debug)]
pub struct AigEdge {
    /// if id is none, it means the node is true
    id: Option<AigNodeId>,
    complement: bool,
}

impl AigEdge {
    fn new(id: AigNodeId, complement: bool) -> Self {
        Self {
            id: Some(id),
            complement,
        }
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
        let inputs = 0..header.i;
        let ands = header.i..header.i + header.a;
        let mut nodes: Vec<AigNode> = Vec::with_capacity(header.m);
        let nodes_remaining = nodes.spare_capacity_mut();
        let mut outputs = Vec::new();
        let mut latchs = Vec::new();
        for obj in aiger.records() {
            let obj = obj.unwrap();
            match obj {
                aiger::Aiger::Input(input) => {
                    nodes_remaining[input.0 / 2 - 1].write(AigNode::new_input());
                }
                aiger::Aiger::Latch { output, input } => {
                    nodes_remaining[output.0 / 2 - 1].write(AigNode::new_input());
                    latchs.push(AigLatch::new(
                        output.0 / 2 - 1,
                        AigEdge::new(input.0 / 2 - 1, input.0 & 0x1 != 0),
                    ))
                }
                aiger::Aiger::Output(o) => outputs.push(AigEdge::new(o.0 / 2 - 1, o.0 & 0x1 != 0)),
                aiger::Aiger::AndGate { output, inputs } => {
                    nodes_remaining[output.0 / 2 - 1].write(AigNode::new_and(
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
