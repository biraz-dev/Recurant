//! Start a run on any floor, to look around it: `cargo run -p zrpg --example
//! visit -- --depth 2`. You arrive at the floor's start as if you had taken
//! the stairs down to it; then it plays as usual (with `ZRPG_SCRIPT`, the
//! scripted input drives it; see `script.rs`). `--calm` keeps the floor's
//! residents asleep, for pictures. Never saves.

use bevy::prelude::*;
use zrpg::{Frame, GameEvent, Run};
use zrpg_core::{Command, EnemyState, Tile};

#[derive(Resource)]
struct Visit {
    depth: u32,
    calm: bool,
    arrived: bool,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let value = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1).cloned())
    };
    let depth = value("--depth").and_then(|d| d.parse().ok()).unwrap_or(2);
    let calm = args.iter().any(|a| a == "--calm");
    let mut app = zrpg::app_unsaved();
    app.insert_resource(Visit {
        depth,
        calm,
        arrived: depth <= 1,
    })
    .add_systems(Update, arrive.in_set(Frame::Command));
    app.run();
}

fn arrive(mut visit: ResMut<Visit>, mut run: ResMut<Run>, mut events: MessageWriter<GameEvent>) {
    if visit.calm {
        for e in run.enemies.iter_mut() {
            e.state = EnemyState::Recover { left: 1.0e9 };
        }
    }
    if visit.arrived {
        return;
    }
    visit.arrived = true;
    // Stand on the stairs of the floor above and go down.
    run.depth = visit.depth - 1;
    let map = &run.floor.map;
    let stairs = map.positions().find(|&p| map.get(p) == Tile::StairsDown);
    if let Some(stairs) = stairs {
        run.player.pos = stairs;
    }
    let mut out = Vec::new();
    run.apply(Command::Descend, &mut out);
    events.write_batch(out.into_iter().map(GameEvent));
    info!("visit: floor {} ({:?})", run.depth, run.floor.place);
}
