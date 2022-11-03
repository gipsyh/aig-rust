use aig::Aig;
use std::time::Instant;

fn main() {
    let start = Instant::now();
    let mut aig = 
        // Aig::from_file("../MC-Benchmark/examples/counter/10bit/counter.aag")
        // Aig::from_file("../MC-Benchmark/hwmcc20/aig/2019/goel/crafted/paper_v3/paper_v3.aag")
        // Aig::from_file("../MC-Benchmark/hwmcc17/single/bj08amba2g1.aag") // nusmv 10s
        // Aig::from_file("../MC-Benchmark/hwmcc17/single/shift1add262144.aag") // good
        Aig::from_file("../MC-Benchmark/hwmcc15/single/vis4arbitp1.aag")// 18s vs 64ms
        // Aig::from_file("../MC-Benchmark/hwmcc19/single/aig/goel/industry/cal9/cal9.aag") // ? vs 30s
        // Aig::from_file("../MC-Benchmark/hwmcc19/single/aig/goel/opensource/vis_arrays_buf_bug/vis_arrays_buf_bug.aag") // ? vs bug 2s
        // Aig::from_file("../MC-Benchmark/hwmcc19/single/aig/goel/opensource/vcegar_QF_BV_itc99_b13_p06/vcegar_QF_BV_itc99_b13_p06.aag") // both fast
        // Aig::from_file("../MC-Benchmark/hwmcc17/single/ringp0.aag")
        // Aig::from_file("../MC-Benchmark/hwmcc20/aig/2019/beem/anderson.3.prop1-back-serstep.aag")
        // Aig::from_file("../MC-Benchmark/hwmcc20/aig/2019/goel/opensource/h_RCU/h_RCU.aag")
        .unwrap();
    println!("{}", aig);
    aig.fraig(false);
    dbg!(aig.symbolic_mc());
    println!("{:?}", start.elapsed());
}
