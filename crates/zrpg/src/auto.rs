//! Auto mode: the game plays itself, using the bot in `zrpg_core::auto`.
//! Toggle with Tab (gamepad: Y / North). Pressing any move or attack takes
//! control back. Start in auto mode with `--auto`, `ZRPG_AUTO=1`, or `?auto`
//! in the page URL on the web.

use bevy::prelude::*;

use crate::input::{Action, Actions};
use crate::{Frame, platform};

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct AutoMode(pub bool);

/// Actions that mean "I'll play now".
const TAKE_OVER: [Action; 7] = [
    Action::Forward,
    Action::Back,
    Action::StrafeLeft,
    Action::StrafeRight,
    Action::TurnLeft,
    Action::TurnRight,
    Action::Attack,
];

pub struct AutoPlugin;

impl Plugin for AutoPlugin {
    fn build(&self, app: &mut App) {
        // A resumed run may already have set it.
        if !app.world().contains_resource::<AutoMode>() {
            app.insert_resource(AutoMode(platform::auto_requested()));
        }
        app.add_systems(Update, toggle.before(Frame::Command));
    }
}

pub fn toggle(actions: Res<Actions>, mut auto: ResMut<AutoMode>) {
    if actions.just_pressed(Action::Auto) {
        auto.0 = !auto.0;
        info!("auto mode {}", if auto.0 { "on" } else { "off" });
    } else if auto.0 && TAKE_OVER.iter().any(|a| actions.just_pressed(*a)) {
        auto.0 = false;
        info!("auto mode off: player took over");
    }
}
