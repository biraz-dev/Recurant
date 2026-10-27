//! Print a few generated floors: `cargo run -p zrpg_core --example ascii [seed]`
use zrpg_core::{Floor, GenParams};

fn main() {
    let seed = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    for depth in 1..=3 {
        let f = Floor::for_depth(seed, depth, &GenParams::default());
        println!(
            "seed {seed}, floor {depth}: start {:?} facing {:?}",
            f.start, f.start_facing
        );
        println!("{}", f.map);
    }
}
