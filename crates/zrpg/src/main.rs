//! ZRPG - an RPG for busy people.
//!
//! Bevy front-end. All rules live in `zrpg_core`; this crate turns them into
//! pixels and turns buttons into commands.

mod autopilot;
mod input;
mod level;
mod screen;
mod textures;
mod view;

use bevy::prelude::*;
use bevy::window::WindowResolution;
use zrpg_core::Game;

/// The running game, owned by Bevy as a resource.
#[derive(Resource, Deref, DerefMut)]
pub struct Run(pub Game);

impl Run {
    pub fn new(seed: u64) -> Self {
        info!("starting run with seed {seed}");
        Run(Game::new(seed))
    }
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
        .add_plugins((
            screen::ScreenPlugin,
            input::InputPlugin,
            level::LevelPlugin,
            view::ViewPlugin,
            autopilot::AutopilotPlugin,
        ))
        .add_systems(Update, restart)
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

/// Instant restart on a fresh seed.
fn restart(actions: Res<input::Actions>, mut run: ResMut<Run>) {
    if actions.just_pressed(input::Action::Restart) {
        *run = Run::new(zrpg_core::rng::mix64(run.run_seed ^ clock_seed()));
    }
}
