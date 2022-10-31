use crate::{Aig, AigEdge, AigNode, AigNodeType};
use std::fmt::Display;

impl Display for AigNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.typ {
            AigNodeType::True => write!(f, "True"),
            AigNodeType::PrimeInput => write!(f, "PI{}", self.id),
            AigNodeType::LatchInput => write!(f, "LI{}", self.id),
            AigNodeType::And(_, _) => write!(f, "A{}", self.id),
        }
    }
}

impl Display for AigEdge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.complement {
            write!(f, "!")?;
        }
        Ok(())
    }
}

impl Display for Aig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "==================")?;
        writeln!(f, "input num: {}", self.inputs.len(),)?;
        writeln!(f, "latch num: {}", self.latchs.len())?;
        writeln!(f, "output num: {}", self.outputs.len())?;
        writeln!(f, "and num: {}", self.num_ands)?;
        writeln!(f, "bad state num: {}", self.bads.len())?;
        writeln!(f, "------------------")?;
        write!(f, "prime inputs:")?;
        for ci in &self.inputs {
            write!(f, " {}", self.nodes[*ci])?;
        }
        writeln!(f, "\n------------------")?;
        for and in self.ands_iter() {
            let fanin0 = and.fanin0();
            let fanin1 = and.fanin1();
            writeln!(
                f,
                "{} = {}{} & {}{}",
                self.nodes[and.node_id()],
                fanin0,
                self.nodes[fanin0.node_id()],
                fanin1,
                self.nodes[fanin1.node_id()]
            )?;
        }
        writeln!(f, "------------------")?;
        writeln!(f, "outputs:")?;
        for idx in 0..self.outputs.len() {
            writeln!(
                f,
                "O{}: {}{}",
                idx + 1,
                self.outputs[idx],
                self.nodes[self.outputs[idx].node_id()]
            )?;
        }
        writeln!(f, "------------------")?;
        writeln!(f, "bad states:")?;
        for idx in 0..self.bads.len() {
            writeln!(
                f,
                "B{}: {}{}",
                idx + 1,
                self.bads[idx],
                self.nodes[self.bads[idx].node_id()]
            )?;
        }
        writeln!(f, "==================")?;
        Ok(())
    }
}
