#![feature(assert_matches, ptr_metadata, unchecked_math, test)]

mod aiger;
mod brute_force;
mod display;
mod eliminate;
mod fraig;
mod migrate;
mod sat;
mod simulate;
mod strash;
mod symbolic_mc;

use fraig::FrAig;
use sat::SatSolver;
use std::{
    assert_matches::assert_matches,
    cmp::Reverse,
    collections::{BinaryHeap, HashMap},
    mem::{swap, take},
    ops::{Index, Not, Range},
    slice::Iter,
    vec,
};

type AigNodeId = usize;

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
    typ: AigNodeType,
    fanouts: Vec<AigEdge>,
}

impl AigNode {
    fn node_id(&self) -> AigNodeId {
        self.id
    }

    fn is_and(&self) -> bool {
        matches!(self.typ, AigNodeType::And(_, _))
    }

    fn is_cinput(&self) -> bool {
        matches!(self.typ, AigNodeType::LatchInput | AigNodeType::PrimeInput)
    }

    fn _is_prime_input(&self) -> bool {
        matches!(self.typ, AigNodeType::PrimeInput)
    }

    fn fanin0(&self) -> AigEdge {
        if let AigNodeType::And(ret, _) = self.typ {
            ret
        } else {
            panic!();
        }
    }

    fn fanin1(&self) -> AigEdge {
        if let AigNodeType::And(_, ret) = self.typ {
            ret
        } else {
            panic!();
        }
    }

    fn set_fanin0(&mut self, fanin: AigEdge) {
        if let AigNodeType::And(fanin0, _) = &mut self.typ {
            *fanin0 = fanin
        } else {
            panic!();
        }
    }

    fn set_fanin1(&mut self, fanin: AigEdge) {
        if let AigNodeType::And(_, fanin1) = &mut self.typ {
            *fanin1 = fanin
        } else {
            panic!();
        }
    }
}

impl AigNode {
    fn new_true(id: usize) -> Self {
        Self {
            id,
            typ: AigNodeType::True,
            fanouts: Vec::new(),
            level: 0,
        }
    }

    fn new_prime_input(id: usize) -> Self {
        Self {
            id,
            typ: AigNodeType::PrimeInput,
            fanouts: Vec::new(),
            level: 0,
        }
    }

    fn new_latch_input(id: usize) -> Self {
        Self {
            id,
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
            typ: AigNodeType::And(fanin0, fanin1),
            fanouts: Vec::new(),
            level,
        }
    }

    fn strash_key(&self) -> (AigEdge, AigEdge) {
        (self.fanin0(), self.fanin1())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
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

impl From<AigNodeId> for AigEdge {
    fn from(value: AigNodeId) -> Self {
        Self {
            id: value,
            complement: false,
        }
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

    pub fn set_nodeid(&mut self, nodeid: AigNodeId) {
        self.id = nodeid;
    }

    pub fn set_compl(&mut self, compl: bool) {
        self.complement = compl
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
    inputs: Vec<AigNodeId>,
    latchs: Vec<AigLatch>,
    outputs: Vec<AigEdge>,
    bads: Vec<AigEdge>,
    num_inputs: usize,
    num_latchs: usize,
    num_ands: usize,
    strash_map: HashMap<(AigEdge, AigEdge), AigNodeId>,
    fraig: Option<FrAig>,
    sat_solver: Box<dyn SatSolver>,
}

impl Aig {
    fn constant_edge(polarity: bool) -> AigEdge {
        AigEdge {
            id: 0,
            complement: !polarity,
        }
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
        // let nodeid = match self.node_gc.alloc_input_node() {
        //     Some(id) => id,
        //     None => self.nodes.len(),
        // };
        let nodeid = self.nodes.len();
        let input = AigNode::new_prime_input(nodeid);
        if let Some(fraig) = &mut self.fraig {
            fraig.new_input_node(nodeid);
        }
        self.nodes.push(input);
        self.inputs.push(nodeid);
        self.num_inputs += 1;
        self.sat_solver.add_input_node(nodeid);
        nodeid
    }

    pub fn new_and_node(&mut self, mut fanin0: AigEdge, mut fanin1: AigEdge) -> AigEdge {
        if fanin0.node_id() > fanin1.node_id() {
            swap(&mut fanin0, &mut fanin1);
        }
        // if let Some(id) = self.strash_map.get(&(fanin0, fanin1)) {
        //     return AigEdge::new(*id, false);
        // }
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
            if self.fraig.is_some() {
                if let Some(and_edge) = self.fraig.as_mut().unwrap().new_and_node(
                    &self.nodes,
                    self.sat_solver.as_mut(),
                    fanin0,
                    fanin1,
                    nodeid,
                ) {
                    return and_edge;
                }
            }
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
            self.sat_solver.add_and_node(nodeid, fanin0, fanin1);
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

    pub fn new_and_nodes(&mut self, edges: Vec<AigEdge>) -> AigEdge {
        assert!(edges.len() > 1);
        let mut heap = BinaryHeap::new();
        for edge in edges {
            heap.push(Reverse((self.nodes[edge.node_id()].level, edge)));
        }
        while heap.len() > 1 {
            let peek0 = heap.pop().unwrap().0 .1;
            let peek1 = heap.pop().unwrap().0 .1;
            let new_node = self.new_and_node(peek0, peek1);
            heap.push(Reverse((self.nodes[new_node.node_id()].level, new_node)));
        }
        heap.pop().unwrap().0 .1
    }

    pub fn add_output(&mut self, out: AigEdge) {
        self.outputs.push(out)
    }

    pub fn merge_fe_node(&mut self, replaced: AigEdge, by: AigEdge) {
        let compl = replaced.compl() != by.compl();
        let replaced = replaced.node_id();
        let by = by.node_id();
        assert!(replaced > by);
        let fanouts = take(&mut self.nodes[replaced].fanouts);
        for fanout in fanouts {
            let fanout_node_id = fanout.node_id();
            let fanout_node = &mut self.nodes[fanout_node_id];
            // self.strash_map.remove(&fanout_node.strash_key()).unwrap();
            let mut fanin0 = fanout_node.fanin0();
            let mut fanin1 = fanout_node.fanin1();
            assert!(fanin0.node_id() < fanin1.node_id());
            if fanin0.node_id() == replaced {
                assert_eq!(fanout.compl(), fanin0.compl());
                fanin0 = AigEdge::new(by, fanout.compl() ^ compl);
            }
            if fanin1.node_id() == replaced {
                assert_eq!(fanout.compl(), fanin1.compl());
                fanin1 = AigEdge::new(by, fanout.compl() ^ compl);
            }
            if fanin0.node_id() > fanin1.node_id() {
                swap(&mut fanin0, &mut fanin1);
            }
            fanout_node.set_fanin0(fanin0);
            fanout_node.set_fanin1(fanin1);
            self.nodes[fanout_node_id].level = self.nodes[fanin0.node_id()]
                .level
                .max(self.nodes[fanin1.node_id()].level)
                + 1;
            self.nodes[by].fanouts.push(fanout);
            let _strash_key = self.nodes[fanout_node_id].strash_key();
            // assert!(self.strash_map.insert(strash_key, fanout_node_id).is_none());
        }
    }
}

impl Aig {
    pub fn num_nodes(&self) -> usize {
        self.nodes.len()
    }

    pub fn nodes_range(&self) -> Range<usize> {
        1..self.num_nodes()
    }

    pub fn nodes_range_with_true(&self) -> Range<usize> {
        0..self.num_nodes()
    }

    pub fn pinputs_iter(&self) -> Iter<AigNode> {
        todo!()
    }

    // pub fn cinputs_iter(&self) -> impl Iterator<Item = &AigNode> {
    //     self.cinputs.iter().map(|id| &self.nodes[*id])
    // }

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

    pub fn fanin_logic_cone(&self, logic: &[AigEdge]) -> Vec<bool> {
        let mut flag = vec![false; self.num_nodes()];
        for l in logic {
            flag[l.node_id()] = true;
        }
        for id in self.nodes_range_with_true().rev() {
            if flag[id] && self.nodes[id].is_and() {
                flag[self.nodes[id].fanin0().node_id()] = true;
                flag[self.nodes[id].fanin1().node_id()] = true;
            }
        }
        flag
    }

    pub fn fanout_logic_cone(&self, logic: AigEdge) -> Vec<bool> {
        let mut flag = vec![false; self.num_nodes()];
        flag[logic.node_id()] = true;
        for id in self.nodes_range_with_true() {
            if flag[id] {
                for f in &self.nodes[id].fanouts {
                    flag[f.node_id()] = true;
                }
            }
        }
        flag
    }
}

impl Aig {
    pub fn cleanup_redundant(&mut self, observes: &[AigEdge]) -> Vec<Option<AigNodeId>> {
        let mut observe = observes.to_vec();
        observe.extend(&self.bads);
        observe.extend(&self.outputs);
        for l in &self.latchs {
            observe.push(l.next);
            observe.push(l.input.into());
        }
        for i in &self.inputs {
            observe.push((*i).into());
        }
        let mut observe = self.fanin_logic_cone(&observe);
        observe[0] = true;
        self.num_ands = 0;
        let old_nodes = take(&mut self.nodes);
        self.sat_solver = Box::new(sat::abc_glucose::Solver::new());
        let mut node_map = vec![None; old_nodes.len()];
        for mut node in old_nodes {
            if observe[node.id] {
                node.fanouts.clear();
                node_map[node.id] = Some(self.nodes.len());
                node.id = self.nodes.len();
                if node.is_and() {
                    let fanin0 = node.fanin0();
                    let fanin1 = node.fanin1();
                    let fanin0 = AigEdge::new(node_map[fanin0.node_id()].unwrap(), fanin0.compl());
                    let fanin1 = AigEdge::new(node_map[fanin1.node_id()].unwrap(), fanin1.compl());
                    node.set_fanin0(fanin0);
                    node.set_fanin1(fanin1);
                    self.nodes[fanin0.node_id()]
                        .fanouts
                        .push(AigEdge::new(node.id, fanin0.compl()));
                    self.nodes[fanin1.node_id()]
                        .fanouts
                        .push(AigEdge::new(node.id, fanin1.compl()));
                    self.num_ands += 1;
                    self.sat_solver
                        .add_and_node(node.id, node.fanin0(), node.fanin1())
                } else if node.is_cinput() {
                    self.sat_solver.add_input_node(node.id);
                }
                self.nodes.push(node);
            }
        }
        self.fraig.as_mut().unwrap().cleanup_redundant(&node_map);
        for latch in &mut self.latchs {
            latch.input = node_map[latch.input].unwrap();
            latch
                .next
                .set_nodeid(node_map[latch.next.node_id()].unwrap());
        }
        for input in &mut self.inputs {
            *input = node_map[*input].unwrap();
        }
        for out in &mut self.outputs {
            out.set_nodeid(node_map[out.node_id()].unwrap());
        }
        for bad in &mut self.bads {
            bad.set_nodeid(node_map[bad.node_id()].unwrap());
        }
        node_map
    }
}

impl Aig {
    pub fn latch_init_equation(&mut self) -> AigEdge {
        let mut equals = Vec::new();
        let latchs = self.latchs.clone();
        for AigLatch {
            input,
            next: _,
            init,
        } in latchs
        {
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
    use crate::Aig;
    #[test]
    fn test_replace_node() {
        let mut aig = Aig::from_file("aigs/i10.aag").unwrap();
        println!("{}", aig);
        aig.fraig();
    }
}
