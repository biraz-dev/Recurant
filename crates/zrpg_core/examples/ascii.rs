//! Print a few generated floors: `cargo run -p zrpg_core --example ascii [seed]`
//!
//! `@` start, `e` enemy, `>` stairs, `#` wall, `.` floor.
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
        let mut rows: Vec<Vec<char>> = f
            .map
            .to_string()
            .lines()
            .map(|l| l.chars().collect())
            .collect();
        rows[f.start.y as usize][f.start.x as usize] = '@';
        for e in &f.enemy_spawns {
            rows[e.y as usize][e.x as usize] = 'e';
        }
        for row in rows {
            println!("{}", row.into_iter().collect::<String>());
        }
        println!();
    }
}
