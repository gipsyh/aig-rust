use crate::Aig;

pub struct FraigParam {
    sim_nwords: usize,
    sim_nrounds: usize,
}

impl Default for FraigParam {
    fn default() -> Self {
        Self {
            sim_nwords: 4,
            sim_nrounds: 4,
        }
    }
}

pub struct FrAig {}

impl FrAig {
    fn do_fraig(aig: &mut Aig, param: FraigParam) {
        for _ in 0..param.sim_nrounds {
            // let map = HashMap::new();
            // let sims = AigSimulation::new(aig, param.sim_nwords);
            // for sim in sims.simulations() {}
        }
        todo!()
    }
}
