//! Cast every spell at a few creatures standing in the ruins' statue
//! chamber, to see the effects in the game itself:
//! `cargo run -p zrpg --example spellbook`. With `--shots DIR`, screenshots
//! of each spell are saved there and it ends after the last.

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use zrpg::{Frame, GameEvent, Run};
use zrpg_core::{Command, EnemyKind, EnemyState, Facing, GridPos, Spell};

/// Seconds per spell.
const ROUND: f32 = 2.4;
/// When in each round to cast, and to take pictures after casting.
const CAST_AT: f32 = 1.6;
const SHOTS: [f32; 2] = [0.06, 0.16];
const PLAYER: GridPos = GridPos::new(14, 12);
/// Two in the line ahead, two off to the sides.
const FOES: [GridPos; 4] = [
    GridPos::new(14, 10),
    GridPos::new(14, 8),
    GridPos::new(12, 9),
    GridPos::new(16, 10),
];
const FROZEN: EnemyState = EnemyState::Recover { left: 1.0e9 };

#[derive(Resource)]
struct Book {
    round: usize,
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
    app.insert_resource(Book {
        round: 0,
        clock: None,
        shots,
    })
    .add_systems(Update, stage.in_set(Frame::Command));
    app.run();
}

fn stage(
    mut commands: Commands,
    time: Res<Time>,
    mut book: ResMut<Book>,
    mut run: ResMut<Run>,
    mut events: MessageWriter<GameEvent>,
    mut exit: MessageWriter<AppExit>,
) {
    // Round 0 only waits for the screen to fade in.
    let warming_up = book.round == 0;
    let spell = Spell::ALL[(book.round + Spell::ALL.len() - 1) % Spell::ALL.len()];
    let first = book.clock.is_none();
    let before = book.clock.unwrap_or(-1.0);
    let clock = book.clock.unwrap_or(0.0) + time.delta_secs();
    book.clock = Some(clock);

    if first {
        // A new floor number makes the game respawn the sprites.
        run.depth += 1;
        let p = &mut run.player;
        p.pos = PLAYER;
        p.facing = Facing::North;
        p.max_hp = 9999;
        p.hp = 9999;
        run.enemies = FOES
            .iter()
            .zip(EnemyKind::ALL)
            .enumerate()
            .map(|(i, (&at, kind))| {
                let mut e = zrpg_core::Enemy::of_kind(i as u32, at, kind);
                e.hp = 999;
                e.max_hp = 999;
                e
            })
            .collect();
        info!("spellbook: {spell:?}");
    }
    for e in run.enemies.iter_mut() {
        e.state = FROZEN;
    }
    let p = &mut run.player;
    p.energy = p.max_energy;

    if !warming_up && before < CAST_AT && clock >= CAST_AT {
        let mut out = Vec::new();
        run.apply(Command::Cast(spell), &mut out);
        events.write_batch(out.into_iter().map(GameEvent));
    }
    if let Some(dir) = book.shots.clone().filter(|_| !warming_up) {
        for (k, after) in SHOTS.iter().enumerate() {
            let at = CAST_AT + after;
            if before < at && clock >= at {
                let name = format!("{spell:?}").to_lowercase();
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(format!("{dir}/{name}-{k}.png")));
            }
        }
    }
    if clock >= ROUND {
        book.round += 1;
        book.clock = None;
        if book.shots.is_some() && book.round > Spell::ALL.len() {
            exit.write(AppExit::Success);
        }
    }
}
