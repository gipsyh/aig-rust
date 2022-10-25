use aig::Aig;

fn main() {
    let mut aig =
        Aig::from_file("/root/MC-Benchmark/hwmcc20/aig/2019/goel/crafted/paper_v3/paper_v3.aag")
            .unwrap();
    println!("{}", aig);
    aig.fraig();
    println!("{}", aig);
    dbg!(aig.symbolic_mc());
}
