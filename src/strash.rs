use crate::Aig;

impl Aig {
    pub fn strash(&mut self) {
        if self.strash_map.is_some() {
            return;
        }
        let strash_map = self.strash_map.as_mut().unwrap();
        for and in self.nodes[self.ands.clone()].iter() {
            assert!(and.fanin0().node_id() < and.fanin1().node_id());
            let key = (
                and.fanin0().node_id(),
                and.fanin0().compl(),
                and.fanin1().node_id(),
                and.fanin1().compl(),
            );
            match strash_map.get(&key) {
                Some(id) => {
                    todo!()
                }
                None => {
                    strash_map.insert(key, and.node_id());
                }
            }
        }
    }
}
