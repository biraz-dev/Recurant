//! Visit the first floor of every stage, to see how each one looks:
//! `cargo run -p zrpg --example tour`. On each floor you stand at the start
//! looking down the longest open line; two pictures a second apart show
//! what moves. With `--shots DIR`,
//! screenshots are saved there and the tour ends after the last floor.
//! `--seed N` picks the run (and so the order of the stages).

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use zrpg::{Frame, GameEvent, Run};
use zrpg_core::{Command, Facing, Tile};

/// The floors to visit: the ruins, then the first floor of each stage.
const DEPTHS: [u32; 7] = [1, 2, 6, 10, 14, 18, 22];
/// Seconds per floor, and when on each to take its two pictures.
const STAY: f32 = 4.0;
const SHOTS: [f32; 2] = [2.6, 3.6];

#[derive(Resource)]
struct Tour {
    stop: usize,
    clock: Option<f32>,
    shots: Option<String>,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let shots = args
        .iter()
        .position(|a| a == "--shots")
        .and_then(|i| args.get(i + 1).cloned());
    let mut app = zrpg::app_unsaved();
    app.insert_resource(Tour {
        stop: 0,
        clock: None,
        shots,
    })
    .add_systems(Update, stage.in_set(Frame::Command));
    app.run();
}

fn stage(
    mut commands: Commands,
    time: Res<Time>,
    mut tour: ResMut<Tour>,
    mut run: ResMut<Run>,
    mut events: MessageWriter<GameEvent>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(&depth) = DEPTHS.get(tour.stop) else {
        if tour.shots.is_some() {
            exit.write(AppExit::Success);
        }
        return;
    };
    let first = tour.clock.is_none();
    let before = tour.clock.unwrap_or(-1.0);
    let clock = tour.clock.unwrap_or(0.0) + time.delta_secs();
    tour.clock = Some(clock);

    if first && depth > 1 {
        // Stand on the stairs of the floor above and go down.
        run.depth = depth - 1;
        let map = &run.floor.map;
        let stairs = map.positions().find(|&p| map.get(p) == Tile::StairsDown);
        if let Some(stairs) = stairs {
            run.player.pos = stairs;
        }
        let mut out = Vec::new();
        run.apply(Command::Descend, &mut out);
        events.write_batch(out.into_iter().map(GameEvent));
    }
    if first {
        // Look down the longest open line.
        let map = &run.floor.map;
        let start = run.player.pos;
        let reach = |f: Facing| {
            let mut at = start;
            let mut n = 0;
            while n < 20 && map.can_step(at, f) {
                at = at + f;
                n += 1;
            }
            n
        };
        let best = Facing::ALL.into_iter().max_by_key(|&f| reach(f));
        if let Some(f) = best {
            run.player.facing = f;
        }
        info!(
            "tour: floor {depth}, {:?}",
            run.floor.stage.map(|s| s.theme)
        );
    }
    let p = &mut run.player;
    p.max_hp = 9999;
    p.hp = 9999;
    // Keep the residents asleep so they don't interrupt.
    run.enemies.clear();

    if let Some(dir) = tour.shots.clone() {
        for (k, at) in SHOTS.iter().enumerate() {
            if before < *at && clock >= *at {
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(format!("{dir}/floor{depth:02}-{k}.png")));
            }
        }
    }
    if clock >= STAY {
        tour.stop += 1;
        tour.clock = None;
    }
}
