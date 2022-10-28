use std::time::Instant;

use aig::Aig;

fn main() {
    let start = Instant::now();
    let mut aig = Aig::from_file("/root/MC-Benchmark/examples/counter/10bit/counter.aag")
        // Aig::from_file("/root/MC-Benchmark/hwmcc20/aig/2019/goel/crafted/paper_v3/paper_v3.aag")
        // Aig::from_file("/root/MC-Benchmark/hwmcc20/aig/2019/goel/opensource/h_RCU/h_RCU.aag")
        .unwrap();
    aig.fraig();
    dbg!(aig.symbolic_mc());
    println!("{:?}", start.elapsed());
}
