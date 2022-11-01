use aig::Aig;
use std::time::Instant;

fn main() {
    let start = Instant::now();
    let mut aig = 
        // Aig::from_file("/root/MC-Benchmark/examples/counter/10bit/counter.aag")
        // Aig::from_file("/root/MC-Benchmark/hwmcc20/aig/2019/goel/crafted/paper_v3/paper_v3.aag")
        // Aig::from_file("/root/MC-Benchmark/hwmcc17/single/bj08amba2g1.aag") // nusmv 10s
        // Aig::from_file("/root/MC-Benchmark/hwmcc17/single/shift1add262144.aag") // good
        Aig::from_file("/root/MC-Benchmark/hwmcc15/single/vis4arbitp1.aag")// 110s vs 64ms
        // Aig::from_file("/root/MC-Benchmark/hwmcc17/single/ringp0.aag")
        // Aig::from_file("/root/MC-Benchmark/hwmcc20/aig/2019/beem/anderson.3.prop1-back-serstep.aag")
        // Aig::from_file("/root/MC-Benchmark/hwmcc20/aig/2019/goel/opensource/h_RCU/h_RCU.aag")
        .unwrap();
    println!("{}", aig);
    aig.fraig(false);
    dbg!(aig.symbolic_mc());
    println!("{:?}", start.elapsed());
}
