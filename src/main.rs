use aig::Aig;
use std::time::Instant;

fn main() {
    let start = Instant::now();
    let mut aig = 
        // Aig::from_file("../MC-Benchmark/examples/counter/10bit/counter.aag")
        // Aig::from_file("../MC-Benchmark/hwmcc20/aig/2019/goel/crafted/paper_v3/paper_v3.aag")
        // Aig::from_file("../MC-Benchmark/hwmcc15/single/vis4arbitp1.aag") // 2.2s vs 0.1ms
        // Aig::from_file("../MC-Benchmark/hwmcc08/viseisenberg.aag")  // 1.8s vs 0.2s
        // Aig::from_file("../MC-Benchmark/hwmcc08/cmugigamax.aag")  // 0.05s vs 0.2s
        // Aig::from_file("../MC-Benchmark/hwmcc08/viselevatorp1.aag")  // 0.004s vs ?
        // Aig::from_file("../MC-Benchmark/hwmcc08/viscoherencep1.aag")  // 0.5s vs 5s
        // Aig::from_file("../MC-Benchmark/hwmcc08/visbakery.aag")  // 29s vs 0.3
        // Aig::from_file("../MC-Benchmark/hwmcc08/texasifetch1p1.aag")  // 0.002s vs ?
        // Aig::from_file("../MC-Benchmark/hwmcc08/texastwoprocp1.aag")  //
        // Aig::from_file("../MC-Benchmark/hwmcc08/srg5ptimo.aag")  //
        // Aig::from_file("../MC-Benchmark/hwmcc08/pdtvishuffman7.aag")  // 5s vs 0.001s
        Aig::from_file("../MC-Benchmark/hwmcc20/aig/2019/goel/opensource/h_TreeArb/h_TreeArb.aag")  // 
        // Aig::from_file("../MC-Benchmark/hwmcc17/single/bj08amba2g1.aag") // 5ms vs nusmv 10s
        // Aig::from_file("../MC-Benchmark/hwmcc17/single/shift1add262144.aag") // ? vs 30s
        // Aig::from_file("../MC-Benchmark/hwmcc19/single/aig/goel/industry/cal9/cal9.aag") // 7s vs 37s
        // Aig::from_file("../MC-Benchmark/hwmcc19/single/aig/goel/opensource/vis_arrays_buf_bug/vis_arrays_buf_bug.aag") // ? vs bug 2s
        // Aig::from_file("../MC-Benchmark/hwmcc17/single/ringp0.aag") // 42s vs 80s
        // Aig::from_file("../MC-Benchmark/hwmcc08/bj08autg3f3.aag")  // both fast
        // Aig::from_file("../MC-Benchmark/hwmcc19/single/aig/goel/opensource/vcegar_QF_BV_itc99_b13_p06/vcegar_QF_BV_itc99_b13_p06.aag") // both fast
        .unwrap();
    println!("{}", aig);
    aig.fraig();
    // println!("{}", aig);
    dbg!(aig.bad_back_sat_smc_without_aig());
    // dbg!(aig.brute_force());
    println!("{:?}", start.elapsed());
}
