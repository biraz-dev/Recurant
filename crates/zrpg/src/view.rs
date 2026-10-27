//! The first-person camera. Commands resolve instantly in `zrpg_core`; this
//! module eases the camera from the old pose to the new one, buffering one
//! command so quick taps are never dropped.

use std::f32::consts::FRAC_PI_2;

use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use zrpg_core::{Command, Event, Facing, GridPos, Step, Turn};

use crate::Run;
use crate::input::{Action, Actions};
use crate::level::{WALL_HEIGHT, cell_center, grid_dir};
use crate::screen::{LowResTarget, world_target};

const STEP_SECS: f32 = 0.16;
const TURN_SECS: f32 = 0.13;
const BUMP_SECS: f32 = 0.14;
const BUMP_DEPTH: f32 = 0.08;
const EYE_HEIGHT: f32 = WALL_HEIGHT * 0.5;
/// Stand a little behind the cell centre so the side walls of the current
/// cell stay in view, as in the classics.
const EYE_BACK: f32 = 0.25;
const FOV_DEGREES: f32 = 60.0;
const FOG_START: f32 = 0.5;
const FOG_END: f32 = 5.5;

pub struct ViewPlugin;

impl Plugin for ViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera)
            .add_systems(Update, (drive_player, animate_camera).chain());
    }
}

#[derive(Component)]
pub struct PlayerCamera;

/// Camera pose: position on the grid plane and yaw in radians (unwrapped, so
/// turning never takes the long way round).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Pose {
    pos: Vec3,
    yaw: f32,
}

impl Pose {
    fn at(cell: GridPos, facing: Facing) -> Self {
        Pose {
            pos: cell_center(cell),
            yaw: yaw_of(facing),
        }
    }

    fn transform(self) -> Transform {
        let rot = Quat::from_rotation_y(self.yaw);
        let back = rot * Vec3::Z * EYE_BACK;
        Transform::from_translation(self.pos + back + Vec3::Y * EYE_HEIGHT).with_rotation(rot)
    }
}

/// Camera looks down -Z at yaw 0 (north); turning right is negative yaw.
fn yaw_of(facing: Facing) -> f32 {
    -(facing.quarter_turns() as f32) * FRAC_PI_2
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Motion {
    Idle,
    Glide { from: Pose, to: Pose },
    Bump { base: Pose, dir: Vec3 },
}

#[derive(Component)]
struct Animator {
    motion: Motion,
    elapsed: f32,
    duration: f32,
    /// Pose once the current motion finishes.
    rest: Pose,
    queued: Option<Command>,
    /// Which run/floor `rest` belongs to; a mismatch snaps the camera.
    synced_to: (u64, u32),
}

impl Animator {
    fn busy(&self) -> bool {
        self.motion != Motion::Idle
    }

    fn start(&mut self, motion: Motion, duration: f32) {
        self.motion = motion;
        self.elapsed = 0.0;
        self.duration = duration;
    }

    fn pose(&self) -> Pose {
        let t = (self.elapsed / self.duration).clamp(0.0, 1.0);
        match self.motion {
            Motion::Idle => self.rest,
            Motion::Glide { from, to } => {
                let e = ease(t);
                Pose {
                    pos: from.pos.lerp(to.pos, e),
                    yaw: from.yaw + (to.yaw - from.yaw) * e,
                }
            }
            Motion::Bump { base, dir } => Pose {
                pos: base.pos + dir * BUMP_DEPTH * (t * std::f32::consts::PI).sin(),
                yaw: base.yaw,
            },
        }
    }
}

fn ease(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn spawn_camera(mut commands: Commands, target: Res<LowResTarget>, run: Res<Run>) {
    let rest = Pose::at(run.player.pos, run.player.facing);
    commands.spawn((
        PlayerCamera,
        Camera3d::default(),
        Camera {
            order: 0,
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        world_target(&target),
        Projection::Perspective(PerspectiveProjection {
            fov: FOV_DEGREES.to_radians(),
            near: 0.05,
            far: FOG_END + 1.0,
            ..default()
        }),
        Tonemapping::None,
        Msaa::Off,
        DistanceFog {
            color: Color::BLACK,
            falloff: FogFalloff::Linear {
                start: FOG_START,
                end: FOG_END,
            },
            ..default()
        },
        rest.transform(),
        Animator {
            motion: Motion::Idle,
            elapsed: 0.0,
            duration: 1.0,
            rest,
            queued: None,
            synced_to: (run.run_seed, run.depth),
        },
    ));
}

fn command_for(a: Action) -> Option<Command> {
    Some(match a {
        Action::Forward => Command::Step(Step::Forward),
        Action::Back => Command::Step(Step::Back),
        Action::StrafeLeft => Command::Step(Step::StrafeLeft),
        Action::StrafeRight => Command::Step(Step::StrafeRight),
        Action::TurnLeft => Command::Turn(Turn::Left),
        Action::TurnRight => Command::Turn(Turn::Right),
        Action::Restart => return None,
    })
}

fn drive_player(
    time: Res<Time>,
    actions: Res<Actions>,
    mut run: ResMut<Run>,
    mut cams: Query<&mut Animator, With<PlayerCamera>>,
) {
    let Ok(mut anim) = cams.single_mut() else {
        return;
    };

    // A new floor or a restart: snap to the new start, forget pending input.
    let floor_id = (run.run_seed, run.depth);
    if anim.synced_to != floor_id {
        anim.synced_to = floor_id;
        anim.rest = Pose::at(run.player.pos, run.player.facing);
        anim.motion = Motion::Idle;
        anim.queued = None;
        return;
    }

    // Fresh presses are buffered (latest wins) so taps during motion count.
    if let Some(cmd) = Action::ALL
        .into_iter()
        .filter(|a| actions.just_pressed(*a))
        .find_map(command_for)
    {
        anim.queued = Some(cmd);
    }

    anim.elapsed += time.delta_secs();
    if anim.busy() && anim.elapsed >= anim.duration {
        anim.motion = Motion::Idle;
    }
    if anim.busy() {
        return;
    }

    // Holding a direction keeps walking.
    let (cmd, from_hold) = match anim.queued.take() {
        Some(cmd) => (cmd, false),
        None => match Action::ALL
            .into_iter()
            .filter(|a| actions.pressed(*a))
            .find_map(command_for)
        {
            Some(cmd) => (cmd, true),
            None => return,
        },
    };

    let from = anim.rest;
    match run.apply(cmd) {
        Event::Moved { to, .. } => {
            let to = Pose {
                pos: cell_center(to),
                yaw: from.yaw,
            };
            anim.rest = to;
            anim.start(Motion::Glide { from, to }, STEP_SECS);
        }
        Event::Turned { turn, .. } => {
            let delta = match turn {
                Turn::Left => FRAC_PI_2,
                Turn::Right => -FRAC_PI_2,
            };
            let to = Pose {
                pos: from.pos,
                yaw: from.yaw + delta,
            };
            anim.rest = to;
            anim.start(Motion::Glide { from, to }, TURN_SECS);
        }
        // Bump only on deliberate presses; holding into a wall stays calm.
        Event::Bumped { dir, .. } if !from_hold => {
            anim.start(
                Motion::Bump {
                    base: from,
                    dir: grid_dir(dir),
                },
                BUMP_SECS,
            );
        }
        Event::Bumped { .. } => {}
    }
}

fn animate_camera(mut cams: Query<(&Animator, &mut Transform), With<PlayerCamera>>) {
    for (anim, mut t) in &mut cams {
        *t = anim.pose().transform();
    }
}
