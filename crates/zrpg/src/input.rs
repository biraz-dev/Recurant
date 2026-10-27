//! Device-agnostic actions. Keyboard and every connected gamepad feed the
//! same `Actions` resource; gameplay code never looks at raw devices.

use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Forward,
    Back,
    StrafeLeft,
    StrafeRight,
    TurnLeft,
    TurnRight,
    Restart,
}

impl Action {
    pub const ALL: [Action; 7] = [
        Action::Forward,
        Action::Back,
        Action::StrafeLeft,
        Action::StrafeRight,
        Action::TurnLeft,
        Action::TurnRight,
        Action::Restart,
    ];

    fn keys(self) -> &'static [KeyCode] {
        use KeyCode::*;
        match self {
            Action::Forward => &[KeyW, ArrowUp, Numpad8],
            Action::Back => &[KeyS, ArrowDown, Numpad2],
            Action::StrafeLeft => &[KeyA, Numpad4],
            Action::StrafeRight => &[KeyD, Numpad6],
            Action::TurnLeft => &[KeyQ, ArrowLeft, Numpad7],
            Action::TurnRight => &[KeyE, ArrowRight, Numpad9],
            Action::Restart => &[KeyR],
        }
    }

    fn buttons(self) -> &'static [GamepadButton] {
        use GamepadButton::*;
        match self {
            Action::Forward => &[DPadUp],
            Action::Back => &[DPadDown],
            Action::StrafeLeft => &[LeftTrigger],
            Action::StrafeRight => &[RightTrigger],
            Action::TurnLeft => &[DPadLeft],
            Action::TurnRight => &[DPadRight],
            Action::Restart => &[Select],
        }
    }

    /// Left stick moves/turns, right stick strafes. Returns `(axis, sign)`.
    fn stick(self) -> Option<(GamepadAxis, f32)> {
        use GamepadAxis::*;
        match self {
            Action::Forward => Some((LeftStickY, 1.0)),
            Action::Back => Some((LeftStickY, -1.0)),
            Action::TurnLeft => Some((LeftStickX, -1.0)),
            Action::TurnRight => Some((LeftStickX, 1.0)),
            Action::StrafeLeft => Some((RightStickX, -1.0)),
            Action::StrafeRight => Some((RightStickX, 1.0)),
            Action::Restart => None,
        }
    }
}

/// Stick deflection that counts as "pressed". Releasing needs a smaller
/// deflection (hysteresis) so a resting thumb doesn't chatter.
const STICK_PRESS: f32 = 0.6;
const STICK_RELEASE: f32 = 0.4;

#[derive(Resource, Default, Debug)]
pub struct Actions {
    held: [bool; Action::ALL.len()],
    just: [bool; Action::ALL.len()],
    /// Action injected by code (autopilot, touch UI later) for one frame.
    injected: Option<Action>,
}

impl Actions {
    pub fn pressed(&self, a: Action) -> bool {
        self.held[a as usize]
    }

    pub fn just_pressed(&self, a: Action) -> bool {
        self.just[a as usize]
    }

    pub fn inject(&mut self, a: Action) {
        self.injected = Some(a);
    }
}

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Actions>()
            .add_systems(PreUpdate, read_devices.after(bevy::input::InputSystems));
    }
}

fn read_devices(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut actions: ResMut<Actions>,
) {
    let injected = actions.injected.take();
    for a in Action::ALL {
        let i = a as usize;
        let was_held = actions.held[i];
        let mut held = a.keys().iter().any(|k| keys.pressed(*k));
        let mut just = a.keys().iter().any(|k| keys.just_pressed(*k));
        for pad in &gamepads {
            held |= a.buttons().iter().any(|b| pad.pressed(*b));
            just |= a.buttons().iter().any(|b| pad.just_pressed(*b));
            if let Some((axis, sign)) = a.stick() {
                let v = pad.get(axis).unwrap_or(0.0) * sign;
                held |= v > STICK_PRESS || (was_held && v > STICK_RELEASE);
            }
        }
        if injected == Some(a) {
            held = true;
            just = true;
        }
        actions.just[i] = just || (held && !was_held);
        actions.held[i] = held;
    }
}
