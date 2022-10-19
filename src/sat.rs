use splr::Certificate;

use crate::{Aig, AigEdge};

impl Aig {
    fn cnf(&self, logic: AigEdge) {}

    pub fn sat(&self, logic: AigEdge) {}
}

#[test]
fn test() {
    let a =
        Certificate::try_from(vec![vec![1, 2], vec![-1, -2], vec![-1, 2], vec![1, -2]]).unwrap();
    dbg!(a);
}
