//! Parade every creature through the ruins' statue chamber, to check its
//! sprite sheet in the game itself: `cargo run -p zrpg --example bestiary`.
//!
//! For each kind, one creature crosses the chamber to the right and back
//! (its right and left views), then another walks up to you (front),
//! attacks, and walks away (back). The core still decides when the blow
//! lands, as in the game. With `--shots DIR`, screenshots of each stage are
//! saved there and the parade ends after the last creature.

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use zrpg::{Frame, GameEvent, Run};
use zrpg_core::{EnemyKind, EnemyState, Event, Facing, GridPos};

/// Seconds per beat: one step, or one pause.
const BEAT: f32 = 0.4;
/// Where you stand in the statue chamber, facing the statue.
const PLAYER: GridPos = GridPos::new(14, 12);
/// The crosser walks this row, west to east and back.
const CROSS_ROW: i32 = 9;
const CROSS_FROM: i32 = 11;
const CROSS_TO: i32 = 16;
/// The walker comes down this column to stand in front of you.
const WALK_FROM: GridPos = GridPos::new(14, 8);
/// Frozen: no core AI between the staged moves.
const FROZEN: EnemyState = EnemyState::Recover { left: 1.0e9 };
const CROSSER: u32 = 0;
const WALKER: u32 = 1;

#[derive(Resource)]
struct Parade {
    kind: usize,
    /// Seconds into this creature's turn; `None` until it is set up.
    clock: Option<f32>,
    shots: Option<String>,
    taken: Vec<(usize, &'static str)>,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let shots = args
        .iter()
        .position(|a| a == "--shots")
        .and_then(|i| args.get(i + 1).cloned());
    let mut app = zrpg::app();
    app.insert_resource(Parade {
        kind: 0,
        clock: None,
        shots,
        taken: Vec::new(),
    })
    .add_systems(Update, stage.in_set(Frame::Command));
    app.run();
}

/// Staged positions, by beat: (crosser, walker).
fn script(beat: i32) -> (GridPos, GridPos) {
    let cross_len = CROSS_TO - CROSS_FROM;
    let crosser = match beat {
        b if b <= cross_len => CROSS_FROM + b,
        b if b <= 2 * cross_len => CROSS_TO - (b - cross_len),
        _ => CROSS_FROM,
    };
    // The walker comes down after the crossing, waits out its attack, and
    // walks back up.
    let down = (beat - 2 * cross_len).clamp(0, 3);
    let up = (beat - (2 * cross_len + 8)).clamp(0, 3);
    let walker = GridPos::new(WALK_FROM.x, WALK_FROM.y + down - up);
    (GridPos::new(crosser, CROSS_ROW), walker)
}

/// Beats per creature.
const BEATS: i32 = 2 * (CROSS_TO - CROSS_FROM) + 12;
/// The beat the walker winds up on (once it stands in front of you).
const ATTACK_BEAT: i32 = 2 * (CROSS_TO - CROSS_FROM) + 4;

fn stage(
    mut commands: Commands,
    time: Res<Time>,
    mut parade: ResMut<Parade>,
    mut run: ResMut<Run>,
    mut events: MessageWriter<GameEvent>,
    mut exit: MessageWriter<AppExit>,
) {
    let kind = EnemyKind::ALL[parade.kind % EnemyKind::ALL.len()];
    let first = parade.clock.is_none();
    let before = parade.clock.map_or(-1, |c| (c / BEAT) as i32);
    let clock = parade.clock.unwrap_or(0.0) + time.delta_secs();
    parade.clock = Some(clock);
    let beat = (clock / BEAT) as i32;

    if first {
        // A new floor number makes the game respawn the sprites.
        run.depth += 1;
        let p = &mut run.player;
        p.pos = PLAYER;
        p.facing = Facing::North;
        p.max_hp = 9999;
        p.hp = 9999;
        let (crosser, walker) = script(0);
        run.enemies = vec![
            zrpg_core::Enemy::of_kind(CROSSER, crosser, kind),
            zrpg_core::Enemy::of_kind(WALKER, walker, kind),
        ];
        info!("bestiary: {kind:?}");
    }
    // Keep everyone frozen except during the staged attack.
    for e in run.enemies.iter_mut() {
        if !e.is_winding_up() {
            e.state = FROZEN;
        }
    }
    if beat != before && beat < BEATS {
        let (crosser, walker) = script(beat);
        for (id, to) in [(CROSSER, crosser), (WALKER, walker)] {
            let Some(e) = run.enemies.iter_mut().find(|e| e.id == id) else {
                continue;
            };
            if e.pos != to {
                let from = e.pos;
                e.pos = to;
                events.write(GameEvent(Event::EnemyMoved { id, from, to }));
            }
        }
        if beat == ATTACK_BEAT
            && let Some(e) = run.enemies.iter_mut().find(|e| e.id == WALKER)
        {
            e.state = EnemyState::WindUp {
                left: e.stats().wind_up,
            };
            events.write(GameEvent(Event::EnemyWindUp { id: WALKER }));
        }
    }

    // Screenshots: mid-stride each way, mid-wind-up, the blow, walking off.
    let cross = (CROSS_TO - CROSS_FROM) as f32;
    let wind_up = kind.stats().wind_up;
    let moments = [
        ("right", (cross * 0.5 + 0.4) * BEAT),
        ("left", (cross * 1.5 + 0.4) * BEAT),
        ("front", (2.0 * cross + 2.4) * BEAT),
        ("windup", ATTACK_BEAT as f32 * BEAT + wind_up * 0.7),
        ("strike", ATTACK_BEAT as f32 * BEAT + wind_up + 0.12),
        ("back", (2.0 * cross + 9.4) * BEAT),
    ];
    if let Some(dir) = parade.shots.clone() {
        for (name, at) in moments {
            let key = (parade.kind, name);
            if clock >= at && !parade.taken.contains(&key) {
                parade.taken.push(key);
                let creature = format!("{kind:?}").to_lowercase();
                let path = format!("{dir}/{creature}-{name}.png");
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(path));
            }
        }
    }

    if beat >= BEATS {
        parade.kind += 1;
        parade.clock = None;
        if parade.shots.is_some() && parade.kind == EnemyKind::ALL.len() {
            exit.write(AppExit::Success);
        }
    }
}
