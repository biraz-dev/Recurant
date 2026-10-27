//! ZRPG - an RPG for busy people.
//!
//! Bevy front-end. All rules live in `zrpg_core`; this crate turns them into
//! pixels and turns buttons into commands. It is a library so each platform
//! can wrap it in its own entry point: `main.rs` on desktop and the web,
//! `zrpg_android` on Android.

mod auto;
mod enemies;
mod hud;
mod input;
mod level;
mod lighting;
mod platform;
mod screen;
mod script;
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

/// The whole game, ready to `run()`.
pub fn app() -> App {
    let seed = platform::seed_override().unwrap_or_else(platform::clock_seed);
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "ZRPG".into(),
                    resolution: WindowResolution::new(screen::WIDTH * 3, screen::HEIGHT * 3),
                    ..platform::window()
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
        lighting::LightingPlugin,
        view::ViewPlugin,
        enemies::EnemiesPlugin,
        auto::AutoPlugin,
        hud::HudPlugin,
        script::ScriptPlugin,
    ))
    .add_systems(
        Update,
        (
            restart.before(Frame::Command),
            simulate.in_set(Frame::Simulate),
        ),
    );
    app
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
/// In auto mode the game picks itself up after this long.
const AUTO_RESTART: f32 = 2.5;

/// Instant restart on a fresh seed: any time with Restart, with Attack once
/// dead, or by itself in auto mode.
fn restart(
    time: Res<Time>,
    actions: Res<Actions>,
    auto: Res<auto::AutoMode>,
    mut run: ResMut<Run>,
    mut dead_for: Local<f32>,
) {
    *dead_for = if run.player.is_alive() {
        0.0
    } else {
        *dead_for + time.delta_secs()
    };
    let rise = (*dead_for > DEATH_LINGER && actions.just_pressed(Action::Attack))
        || (auto.0 && *dead_for > AUTO_RESTART);
    if actions.just_pressed(Action::Restart) || rise {
        *run = Run::new(zrpg_core::rng::mix64(run.run_seed ^ platform::clock_seed()));
    }
}
