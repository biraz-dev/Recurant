//! Scripted input for headless smoke tests and screenshots.
//!
//! `ZRPG_AUTOPILOT="w w e shot d shot quit"` presses one action per tick
//! (`w s a d q e r`), idles on `wait`, saves `zrpg-N.png` on `shot`, and exits on `quit`.
//! Does nothing when the variable is unset.

use std::collections::VecDeque;

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

use crate::input::{Action, Actions};

const TICK_SECS: f32 = 0.4;

pub struct AutopilotPlugin;

impl Plugin for AutopilotPlugin {
    fn build(&self, app: &mut App) {
        if let Ok(script) = std::env::var("ZRPG_AUTOPILOT") {
            let steps = script.split_whitespace().map(str::to_owned).collect();
            app.insert_resource(Autopilot {
                steps,
                timer: Timer::from_seconds(TICK_SECS, TimerMode::Repeating),
                shots: 0,
            })
            .add_systems(Update, tick);
        }
    }
}

#[derive(Resource)]
struct Autopilot {
    steps: VecDeque<String>,
    timer: Timer,
    shots: u32,
}

fn tick(
    mut commands: Commands,
    time: Res<Time>,
    mut pilot: ResMut<Autopilot>,
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
        "r" => Action::Restart,
        "shot" => {
            let path = format!("zrpg-{}.png", pilot.shots);
            info!(
                "autopilot: {path} at {:?} facing {:?}",
                run.player.pos, run.player.facing
            );
            pilot.shots += 1;
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path));
            return;
        }
        "wait" => return,
        "quit" => {
            exit.write(AppExit::Success);
            return;
        }
        other => {
            warn!("autopilot: unknown step {other:?}");
            return;
        }
    };
    actions.inject(action);
}
