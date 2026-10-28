//! ZRPG - an RPG for busy people.
//!
//! Bevy front-end. All rules live in `zrpg_core`; this crate turns them into
//! pixels and turns buttons into commands. It is a library so each platform
//! can wrap it in its own entry point: `main.rs` on desktop and the web,
//! `zrpg_android` on Android.

mod atmosphere;
mod audio;
mod auto;
mod enemies;
mod hud;
mod input;
mod level;
mod lighting;
mod persist;
mod picker;
mod platform;
mod ruins;
mod screen;
mod script;
mod sheets;
pub mod synth;
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

/// The whole game, ready to `run()`. Resumes the saved run, if any.
pub fn app() -> App {
    build(true)
}

/// The game without saving or resuming, for tools and examples.
pub fn app_unsaved() -> App {
    build(false)
}

fn build(saves: bool) -> App {
    let chosen_seed = platform::seed_override();
    // Scripted and seeded runs are for testing: they must not resume, nor
    // overwrite the player's run (unless given a save folder of their own).
    let own_folder = std::env::var_os("ZRPG_SAVE_DIR").is_some();
    let testing = chosen_seed.is_some() || std::env::var_os("ZRPG_SCRIPT").is_some();
    let saves = saves && (own_folder || !testing);
    let resumed = if saves { persist::resume() } else { None };
    let seed = chosen_seed.unwrap_or_else(platform::clock_seed);
    let mut app = App::new();
    match resumed {
        Some(r) => {
            info!(
                "resuming run {} on floor {} at level {}",
                r.game.run_seed, r.game.depth, r.game.player.level
            );
            app.insert_resource(Run(r.game))
                .insert_resource(auto::AutoMode(r.auto || platform::auto_requested()));
        }
        None => {
            app.insert_resource(Run::new(seed));
        }
    }
    if saves {
        app.insert_resource(persist::Saving);
    }
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
    .add_message::<GameEvent>()
    .configure_sets(
        Update,
        (Frame::Command, Frame::Simulate, Frame::Present).chain(),
    )
    .add_plugins((
        screen::ScreenPlugin,
        input::InputPlugin,
        atmosphere::AtmospherePlugin,
        audio::SoundPlugin,
        level::LevelPlugin,
        lighting::LightingPlugin,
        view::ViewPlugin,
        enemies::EnemiesPlugin,
        auto::AutoPlugin,
        hud::HudPlugin,
        picker::PickerPlugin,
        persist::PersistPlugin,
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
    picker: Res<picker::Picker>,
    mut run: ResMut<Run>,
    mut writer: MessageWriter<GameEvent>,
    mut buf: Local<Vec<Event>>,
) {
    // The world holds still while you choose an ability.
    if picker.holds_the_game() {
        return;
    }
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
