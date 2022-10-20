use crate::{Aig, AigEdge};
use std::collections::HashMap;

impl Aig {
    pub fn fraig(&mut self) {
        let simulation = self.new_simulation(10);
        let mut ec_map = HashMap::new();
        for idx in self.nodes_range() {
            match ec_map.get(&simulation.simulations()[idx]) {
                Some(fenode) => {
                    let x = AigEdge::new(idx, false);
                    let y = AigEdge::new(*fenode, false);
                    if self.equivalence_check(x, y) {
                        self.replace_fe_node(idx, *fenode);
                    }
                }
                None => {
                    ec_map.insert(simulation.simulations()[idx].clone(), idx);
                }
            }
        }
    }
}
