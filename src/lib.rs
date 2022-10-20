#![feature(assert_matches)]

mod aiger;
mod display;
mod eliminate;
mod fraig;
mod migrate;
mod sat;
mod simulate;
mod strash;
mod symbolic_mc;

use std::{
    assert_matches::assert_matches,
    collections::HashMap,
    fmt::Display,
    mem::{swap, take},
    ops::{Index, Not},
    slice::Iter,
    vec,
};

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
    pub fn node_id(&self) -> AigNodeId {
        self.id
    }

    pub fn is_and(&self) -> bool {
        matches!(self.typ, AigNodeType::And(_, _))
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
}

impl Into<AigEdge> for AigNode {
    fn into(self) -> AigEdge {
        AigEdge::new(self.id, false)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
        Self { id, complement }
    }

    pub fn node_id(&self) -> AigNodeId {
        self.id
    }

    pub fn compl(&self) -> bool {
        self.complement
    }
}

#[derive(Debug, Clone)]
pub struct AigLatch {
    input: AigNodeId,
    next: AigEdge,
    init: bool,
}

impl AigLatch {
    fn new(input: AigNodeId, next: AigEdge, init: bool) -> Self {
        Self { input, next, init }
    }
}

#[derive(Debug)]
pub struct Aig {
    nodes: Vec<AigNode>,
    cinputs: Vec<AigNodeId>,
    latchs: Vec<AigLatch>,
    outputs: Vec<AigEdge>,
    bads: Vec<AigEdge>,
    num_inputs: usize,
    num_latchs: usize,
    num_ands: usize,
    strash_map: HashMap<(AigEdge, AigEdge), AigNodeId>,
}

impl Aig {
    fn constant_edge(polarity: bool) -> AigEdge {
        AigEdge {
            id: 0,
            complement: !polarity,
        }
    }

    fn node_is_valid(&self, node: AigNodeId) -> bool {
        self.nodes.len() > node
    }
}

impl Aig {
    // fn new() -> Self {
    //     Self {
    //         nodes: vec![AigNode::new_true(0)],
    //         latchs: Vec::new(),
    //         outputs: Vec::new(),
    //         strash_map: HashMap::new(),
    //         cinputs: todo!(),
    //         num_inputs: 0,
    //         num_latchs: 0,
    //         num_ands: 0,
    //         bads: todo!(),
    //     }
    // }

    pub fn new_input_node(&mut self) -> AigNodeId {
        let nodeid = self.nodes.len();
        let input = AigNode::new_prime_input(nodeid);
        self.nodes.push(input);
        self.cinputs.push(nodeid);
        self.num_inputs += 1;
        nodeid
    }

    pub fn new_and_node(&mut self, mut fanin0: AigEdge, mut fanin1: AigEdge) -> AigEdge {
        if fanin0.node_id() > fanin1.node_id() {
            swap(&mut fanin0, &mut fanin1);
        }
        if let Some(id) = self.strash_map.get(&(fanin0, fanin1)) {
            return AigEdge::new(*id, false);
        }
        assert!(self.node_is_valid(fanin0.node_id()) && self.node_is_valid(fanin1.node_id()));
        if fanin0 == Aig::constant_edge(true) {
            return fanin1;
        }
        if fanin0 == Aig::constant_edge(false) {
            return Aig::constant_edge(false);
        }
        if fanin1 == Aig::constant_edge(true) {
            return fanin0;
        }
        if fanin1 == Aig::constant_edge(false) {
            return Aig::constant_edge(false);
        }
        if fanin0 == fanin1 {
            fanin0
        } else if fanin0 == !fanin1 {
            Aig::constant_edge(false)
        } else {
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
    }

    pub fn new_or_node(&mut self, fanin0: AigEdge, fanin1: AigEdge) -> AigEdge {
        !self.new_and_node(!fanin0, !fanin1)
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

    pub fn add_output(&mut self, out: AigEdge) {
        self.outputs.push(out)
    }
}

impl Aig {
    pub fn num_nodes(&self) -> usize {
        self.nodes.len()
    }

    // pub fn outputs(&self) -> &[AigEdge] {
    //     &self.outputs
    // }

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

    pub fn logic_cone(&self, logic: AigEdge) -> Vec<bool> {
        let mut flag = vec![false; self.num_nodes()];
        flag[logic.node_id()] = true;
        for id in (0..self.num_nodes()).rev() {
            if flag[id] {
                if self.nodes[id].is_and() {
                    flag[self.nodes[id].fanin0().node_id()] = true;
                    flag[self.nodes[id].fanin1().node_id()] = true;
                }
            }
        }
        flag
    }
}

impl Aig {
    pub fn latch_init_equation(&mut self) -> AigEdge {
        let mut equals = Vec::new();
        let latchs = self.latchs.clone();
        for AigLatch { input, next, init } in latchs {
            let init_equal_node = self.new_equal_node((input).into(), Aig::constant_edge(init));
            equals.push(init_equal_node);
        }
        self.new_and_nodes(equals)
    }

    pub fn transfer_latch_outputs_into_pinputs(
        &mut self,
    ) -> (Vec<(AigNodeId, AigNodeId)>, AigEdge) {
        let latchs = take(&mut self.latchs);
        self.num_latchs = 0;
        let mut ret = Vec::new();
        let mut equals = Vec::new();
        for AigLatch {
            input,
            next,
            init: _,
        } in latchs
        {
            assert_matches!(self.nodes[input].typ, AigNodeType::LatchInput);
            self.nodes[input].typ = AigNodeType::PrimeInput;
            self.num_inputs += 1;
            let inode = self.new_input_node();
            ret.push((inode, input));
            let equal_node = self.new_equal_node(next, inode.into());
            equals.push(equal_node);
        }
        (ret, self.new_and_nodes(equals))
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
    use crate::{Aig, AigEdge};
    #[test]
    fn test_from_file() {
        let mut aig = Aig::from_file("aigs/counter-3bit.aag").unwrap();
        println!("{}", aig);
        // aig.eliminate_input(1);
        // println!("{}", aig);
    }

    #[test]
    fn setup_transition() {
        let mut aig = Aig::from_file("aigs/counter_init11.aag").unwrap();
        println!("{}", aig);
        let reachable = aig.latch_init_equation();
        println!("{}", aig);
        let (_, equation) = aig.transfer_latch_outputs_into_pinputs();
        let mut equation = aig.new_and_node(reachable, equation);
        println!("{}", aig);
        equation = aig.eliminate_input(1, vec![equation])[0];
        equation = aig.eliminate_input(2, vec![equation])[0];
        println!("{}", aig);
        dbg!(equation);
        let constraint = aig.migrate_logic(&vec![(7, 1), (11, 2)], AigEdge::new(40, false));
        println!("{}", aig);
    }

    #[test]

    fn symbolic_mc() {
        let mut aig = Aig::from_file("aigs/counter_init11.aag").unwrap();
        aig.symbolic_mc();
    }
}
