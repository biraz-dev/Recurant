//! Scripted input for headless smoke tests and screenshots. (Not to be
//! confused with auto mode, which plays the game for real.)
//!
//! `ZRPG_SCRIPT="w w e shot d shot quit"` presses one action per tick
//! (`w s a d q e f r`, `1` for the item slot, `z x c b n g` for spells, `v` to shoot, or
//! `auto`, `mute`, `map`, `gear`),
//! idles on `wait`, saves `zrpg-N.png` on `shot`, and exits on `quit`.
//! `slow` runs the game at a twentieth of its speed (steps keep their real
//! pace), to catch quick effects like spells on screen; `fast` undoes it.
//! Does nothing when the variable is unset.

use std::collections::VecDeque;

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

use crate::input::{Action, Actions};

const TICK_SECS: f32 = 0.4;

pub struct ScriptPlugin;

impl Plugin for ScriptPlugin {
    fn build(&self, app: &mut App) {
        if let Ok(script) = std::env::var("ZRPG_SCRIPT") {
            let steps = script.split_whitespace().map(str::to_owned).collect();
            app.insert_resource(Script {
                steps,
                timer: Timer::from_seconds(TICK_SECS, TimerMode::Repeating),
                shots: 0,
            })
            .add_systems(Update, tick);
        }
    }
}

#[derive(Resource)]
struct Script {
    steps: VecDeque<String>,
    timer: Timer,
    shots: u32,
}

fn tick(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut virtual_time: ResMut<Time<Virtual>>,
    mut pilot: ResMut<Script>,
    mut actions: ResMut<Actions>,
    mut exit: MessageWriter<AppExit>,
    run: Res<crate::Run>,
) {
    if !pilot.timer.tick(time.delta()).just_finished() {
        return;
    }
    let Some(step) = pilot.steps.pop_front() else {
        return;
    };
    let action = match step.as_str() {
        "w" => Action::Forward,
        "s" => Action::Back,
        "a" => Action::StrafeLeft,
        "d" => Action::StrafeRight,
        "q" => Action::TurnLeft,
        "e" => Action::TurnRight,
        "f" => Action::Attack,
        "auto" => Action::Auto,
        "mute" => Action::Mute,
        "map" => Action::Map,
        "1" => Action::Item1,
        "z" => Action::Spell1,
        "x" => Action::Spell2,
        "c" => Action::Spell3,
        "b" => Action::Spell4,
        "n" => Action::Spell5,
        "g" => Action::Spell6,
        "gear" => Action::Gear,
        "v" => Action::Shoot,
        "r" => Action::Restart,
        "shot" => {
            let path = format!("zrpg-{}.png", pilot.shots);
            info!(
                "script: {path} at {:?} facing {:?}",
                run.player.pos, run.player.facing
            );
            pilot.shots += 1;
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path));
            return;
        }
        "wait" => return,
        "slow" | "fast" => {
            virtual_time.set_relative_speed(if step == "slow" { 0.05 } else { 1.0 });
            return;
        }
        "quit" => {
            exit.write(AppExit::Success);
            return;
        }
        other => {
            warn!("script: unknown step {other:?}");
            return;
        }
    };
    actions.inject(action);
}
