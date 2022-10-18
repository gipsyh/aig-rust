#![feature(assert_matches)]

use std::{
    assert_matches::assert_matches,
    collections::HashMap,
    fmt::Display,
    io,
    mem::{swap, take},
    ops::{Index, Not},
    path::Path,
    slice::Iter,
    vec,
};

mod eliminate;
mod fraig;
mod simulate;
mod strash;

type AigNodeId = usize;

impl Into<AigEdge> for AigNodeId {
    fn into(self) -> AigEdge {
        AigEdge::new(self, false)
    }
}

#[derive(Debug, Clone)]
pub enum AigNodeType {
    True,
    PrimeInput,
    LatchInput,
    And(AigEdge, AigEdge),
}

#[derive(Debug)]
pub struct AigNode {
    id: AigNodeId,
    level: usize,
    size: usize,
    typ: AigNodeType,
    fanouts: Vec<AigEdge>,
}

impl AigNode {
    fn new_true(id: usize) -> Self {
        Self {
            id,
            size: 0,
            typ: AigNodeType::True,
            fanouts: Vec::new(),
            level: 0,
        }
    }

    fn new_prime_input(id: usize) -> Self {
        Self {
            id,
            size: 0,
            typ: AigNodeType::PrimeInput,
            fanouts: Vec::new(),
            level: 0,
        }
    }

    fn new_latch_input(id: usize) -> Self {
        Self {
            id,
            size: 0,
            typ: AigNodeType::LatchInput,
            fanouts: Vec::new(),
            level: 0,
        }
    }

    fn new_and(id: usize, mut fanin0: AigEdge, mut fanin1: AigEdge, level: usize) -> Self {
        if fanin0.node_id() > fanin1.node_id() {
            swap(&mut fanin0, &mut fanin1);
        }
        Self {
            id,
            size: 0,
            typ: AigNodeType::And(fanin0, fanin1),
            fanouts: Vec::new(),
            level,
        }
    }

    pub fn node_id(&self) -> AigNodeId {
        self.id
    }

    pub fn fanin0(&self) -> AigEdge {
        if let AigNodeType::And(ret, _) = self.typ {
            ret
        } else {
            panic!();
        }
    }

    pub fn fanin1(&self) -> AigEdge {
        if let AigNodeType::And(_, ret) = self.typ {
            ret
        } else {
            panic!();
        }
    }
}

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

impl Into<AigEdge> for AigNode {
    fn into(self) -> AigEdge {
        AigEdge::new(self.id, false)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AigEdge {
    id: AigNodeId,
    complement: bool,
}

impl Not for AigEdge {
    type Output = AigEdge;

    fn not(mut self) -> Self::Output {
        self.complement = !self.complement;
        self
    }
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

impl Display for AigEdge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.complement {
            write!(f, "!")?;
        }
        Ok(())
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
    cinputs: Vec<AigNodeId>,
    latchs: Vec<AigLatch>,
    outputs: Vec<AigEdge>,
    num_inputs: usize,
    num_latchs: usize,
    num_ands: usize,
    strash_map: HashMap<(AigNodeId, bool, AigNodeId, bool), AigNodeId>,
}

impl Aig {
    fn node_is_valid(&self, node: AigNodeId) -> bool {
        self.nodes.len() > node
    }
}

impl Aig {
    fn new() -> Self {
        Self {
            nodes: vec![AigNode::new_true(0)],
            latchs: Vec::new(),
            outputs: Vec::new(),
            strash_map: HashMap::new(),
            cinputs: todo!(),
            num_inputs: 0,
            num_latchs: 0,
            num_ands: 0,
        }
    }

    pub fn new_input_node(&mut self) -> AigNodeId {
        let nodeid = self.nodes.len();
        let input = AigNode::new_prime_input(nodeid);
        self.nodes.push(input);
        self.cinputs.push(nodeid);
        self.num_inputs += 1;
        nodeid
    }

    pub fn new_and_node(&mut self, fanin0: AigEdge, fanin1: AigEdge) -> AigEdge {
        assert!(self.node_is_valid(fanin0.node_id()) && self.node_is_valid(fanin1.node_id()));
        let nodeid = self.nodes.len();
        let level = self.nodes[fanin0.node_id()]
            .level
            .max(self.nodes[fanin1.node_id()].level)
            + 1;
        let and = AigNode::new_and(nodeid, fanin0, fanin1, level);
        self.nodes.push(and);
        self.num_ands += 1;
        self.nodes[fanin0.id]
            .fanouts
            .push(AigEdge::new(nodeid, fanin0.compl()));
        self.nodes[fanin1.id]
            .fanouts
            .push(AigEdge::new(nodeid, fanin1.compl()));
        nodeid.into()
    }

    pub fn new_equal_node(&mut self, fanin0: AigEdge, fanin1: AigEdge) -> AigEdge {
        let node1 = self.new_and_node(fanin0, !fanin1);
        let node2 = self.new_and_node(!fanin0, fanin1);
        let edge1 = !node1;
        let edge2 = !node2;
        self.new_and_node(edge1, edge2)
    }

    pub fn new_and_nodes(&mut self, nodes: Vec<AigEdge>) -> AigEdge {
        assert!(nodes.len() > 1);
        let mut ret = nodes[0];
        for node in &nodes[1..] {
            ret = self.new_and_node(ret, *node)
        }
        ret
    }
}

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
        let mut nodes: Vec<AigNode> = Vec::with_capacity(header.m + 1);
        let nodes_remaining = nodes.spare_capacity_mut();
        nodes_remaining[0].write(AigNode::new_true(0));
        let mut outputs = Vec::new();
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
                aiger::Aiger::Latch { output, input } => {
                    let id = output.0 / 2;
                    nodes_remaining[id].write(AigNode::new_latch_input(id));
                    latchs.push(AigLatch::new(
                        id,
                        AigEdge::new(input.0 / 2, input.0 & 0x1 != 0),
                    ));
                    cinputs.push(id);
                }
                aiger::Aiger::Output(o) => outputs.push(AigEdge::new(o.0 / 2 - 1, o.0 & 0x1 != 0)),
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
                    type_spec,
                    position,
                    symbol,
                } => todo!(),
            }
        }
        unsafe { nodes.set_len(header.m + 1) };
        let mut ret = Self {
            nodes,
            cinputs,
            latchs,
            outputs,
            num_inputs: header.i,
            num_latchs: header.l,
            num_ands: header.a,
            strash_map: HashMap::new(),
        };
        ret.setup_levels();
        ret.setup_fanouts();
        Ok(ret)
    }
}

impl Aig {
    fn num_cinputs(&self) -> usize {
        self.cinputs.len()
    }

    fn num_ands(&self) -> usize {
        self.nodes.len() - self.cinputs.len()
    }

    pub fn num_nodes(&self) -> usize {
        self.nodes.len()
    }

    pub fn pinputs_iter(&self) -> Iter<AigNode> {
        todo!()
    }

    pub fn cinputs_iter(&self) -> impl Iterator<Item = &AigNode> {
        self.cinputs.iter().map(|id| &self.nodes[*id])
    }

    // pub fn cinputs_iter_mut(&mut self) -> impl Iterator<Item = &mut AigNode> {
    //     let a = self.cinputs.clone();
    //     a.iter().map(|id| &mut self.nodes[*id])
    // }

    pub fn ands_iter(&self) -> impl Iterator<Item = &AigNode> {
        self.nodes
            .iter()
            .filter(|node| matches!(node.typ, AigNodeType::And(_, _)))
    }

    pub fn ands_iter_mut(&mut self) -> impl Iterator<Item = &mut AigNode> {
        self.nodes
            .iter_mut()
            .filter(|node| matches!(node.typ, AigNodeType::And(_, _)))
    }
}

impl Aig {
    pub fn merge_latch_outputs_into_pinputs(&mut self) -> (Vec<(AigNodeId, AigNodeId)>, AigEdge) {
        let latchs = take(&mut self.latchs);
        self.num_latchs = 0;
        let mut ret = Vec::new();
        let mut equals = Vec::new();
        for AigLatch { input, next } in latchs {
            assert_matches!(self.nodes[input].typ, AigNodeType::LatchInput);
            self.nodes[input].typ = AigNodeType::PrimeInput;
            self.num_inputs += 1;
            let inode = self.new_input_node();
            ret.push((input, inode));
            let equal_node = self.new_equal_node(next, inode.into());
            equals.push(equal_node);
        }
        let retedge = self.new_and_nodes(equals);
        (ret, retedge)
    }

    fn replace_node(&mut self, src_node: AigNodeId, dst_node: AigNodeId, compl: bool) {
        todo!()
    }
}

impl Index<AigNodeId> for Aig {
    type Output = AigNode;

    fn index(&self, index: AigNodeId) -> &Self::Output {
        &self.nodes[index]
    }
}

impl Display for Aig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "==================")?;
        writeln!(f, "input num: {}", self.num_inputs,)?;
        writeln!(f, "latch num: {}", self.latchs.len())?;
        writeln!(f, "and num: {}", self.num_ands)?;
        writeln!(f, "output num: {}", self.outputs.len())?;
        writeln!(f, "------------------")?;
        write!(f, "cinputs:")?;
        for ci in &self.cinputs {
            write!(f, " I{}", *ci)?;
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
        writeln!(f, "==================")?;
        Ok(())
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

    #[test]
    fn setup_transition() {
        let mut aig = Aig::from_file("aigs/counter.aag").unwrap();
        aig.merge_latch_outputs_into_pinputs();
        println!("{}", aig);
        dbg!(aig);
    }
}
