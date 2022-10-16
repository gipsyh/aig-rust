use std::{io, ops::Range, path::Path};

pub struct AigEdge {
    id: usize,
    complement: bool,
}

impl AigEdge {
    fn new(id: usize, complement: bool) -> Self {
        Self { id, complement }
    }
}

pub struct AigObj {
    fanin0: Option<AigEdge>,
    fanin1: Option<AigEdge>,
}

impl AigObj {
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

pub struct Aig {
    objs: Vec<AigObj>,
    outputs: Vec<AigEdge>,
    inputs: Range<usize>,
    ands: Range<usize>,
}

impl Aig {
    pub fn new(
        objs: Vec<AigObj>,
        outputs: Vec<AigEdge>,
        inputs: Range<usize>,
        ands: Range<usize>,
    ) -> Self {
        Self {
            objs,
            outputs,
            inputs,
            ands,
        }
    }

    pub fn from_file<P: AsRef<Path>>(file: P) -> io::Result<Self> {
        let file = std::fs::File::open(file)?;
        let aiger = aiger::Reader::from_reader(file).unwrap();
        let header = aiger.header();
        assert!(header.l == 0);
        let inputs = 1..header.i + 1;
        let ands = header.i + 1..header.i + header.a + 1;
        let mut objs: Vec<AigObj> = Vec::with_capacity(header.m + 1);
        unsafe { objs.set_len(header.m + 1) };
        let mut outputs = Vec::new();
        for obj in aiger.records() {
            let obj = obj.unwrap();
            match obj {
                aiger::Aiger::Input(input) => objs[input.0 / 2] = AigObj::new_input(),
                aiger::Aiger::Latch { output, input } => todo!(),
                aiger::Aiger::Output(o) => outputs.push(AigEdge::new(o.0 / 2, o.0 & 0x1 != 0)),
                aiger::Aiger::AndGate { output, inputs } => {
                    objs[output.0 / 2] = AigObj::new_and(
                        AigEdge::new(inputs[0].0 / 2, inputs[0].0 & 0x1 != 0),
                        AigEdge::new(inputs[1].0 / 2, inputs[1].0 & 0x1 != 0),
                    )
                }
                aiger::Aiger::Symbol {
                    type_spec,
                    position,
                    symbol,
                } => todo!(),
            }
        }

        Ok(Self::new(objs, outputs, inputs, ands))
    }

    pub fn top_sort(&mut self) {}
}

#[cfg(test)]
mod tests {
    use crate::Aig;
    #[test]
    fn test_from_file() {
        let aig = Aig::from_file("xor.aag").unwrap();
    }
}
