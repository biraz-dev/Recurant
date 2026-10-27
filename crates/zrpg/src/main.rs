//! ZRPG - an RPG for busy people.
//!
//! Bevy front-end. All rules live in `zrpg_core`; this crate turns them into
//! pixels and turns buttons into commands.

mod autopilot;
mod enemies;
mod hud;
mod input;
mod level;
mod screen;
mod textures;
mod view;

use bevy::prelude::*;
use bevy::window::WindowResolution;
use zrpg_core::{Event, Game};

use crate::input::{Action, Actions};

/// The running game, owned by Bevy as a resource.
#[derive(Resource, Deref, DerefMut)]
pub struct Run(pub Game);

impl Run {
    pub fn new(seed: u64) -> Self {
        info!("starting run with seed {seed}");
        Run(Game::new(seed))
    }
}

/// Every core event, re-broadcast for presentation systems.
#[derive(Message, Clone, Copy, Debug)]
pub struct GameEvent(pub Event);

/// Per-frame order: read input and issue commands, advance the simulation,
/// then let presentation react to what happened.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Frame {
    Command,
    Simulate,
    Present,
}

fn main() {
    let seed = run_seed();
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "ZRPG".into(),
                        resolution: WindowResolution::new(screen::WIDTH * 3, screen::HEIGHT * 3),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Run::new(seed))
        .add_message::<GameEvent>()
        .configure_sets(
            Update,
            (Frame::Command, Frame::Simulate, Frame::Present).chain(),
        )
        .add_plugins((
            screen::ScreenPlugin,
            input::InputPlugin,
            level::LevelPlugin,
            view::ViewPlugin,
            enemies::EnemiesPlugin,
            hud::HudPlugin,
            autopilot::AutopilotPlugin,
        ))
        .add_systems(
            Update,
            (
                restart.before(Frame::Command),
                simulate.in_set(Frame::Simulate),
            ),
        )
        .run();
}

/// Seed from `--seed N`, else `ZRPG_SEED`, else the clock.
fn run_seed() -> u64 {
    let mut args = std::env::args().skip_while(|a| a != "--seed").skip(1);
    args.next()
        .or_else(|| std::env::var("ZRPG_SEED").ok())
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(clock_seed)
}

fn clock_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

/// Forward core events to Bevy, emptying `buf`.
pub fn publish(buf: &mut Vec<Event>, writer: &mut MessageWriter<GameEvent>) {
    writer.write_batch(buf.drain(..).map(GameEvent));
}

fn simulate(
    time: Res<Time>,
    mut run: ResMut<Run>,
    mut writer: MessageWriter<GameEvent>,
    mut buf: Local<Vec<Event>>,
) {
    // Clamp so a long stall (window drag, breakpoint) can't skip a whole fight.
    let dt = time.delta_secs().min(0.1);
    run.tick(dt, &mut buf);
    publish(&mut buf, &mut writer);
}

/// Seconds of death before Attack restarts, so a held button can't skip it.
const DEATH_LINGER: f32 = 0.8;

/// Instant restart on a fresh seed: any time with Restart, or with Attack
/// once dead.
fn restart(time: Res<Time>, actions: Res<Actions>, mut run: ResMut<Run>, mut dead_for: Local<f32>) {
    *dead_for = if run.player.is_alive() {
        0.0
    } else {
        *dead_for + time.delta_secs()
    };
    let rise = *dead_for > DEATH_LINGER && actions.just_pressed(Action::Attack);
    if actions.just_pressed(Action::Restart) || rise {
        *run = Run::new(zrpg_core::rng::mix64(run.run_seed ^ clock_seed()));
    }
}
