//! The first-person camera. Commands resolve instantly in `zrpg_core`; this
//! module eases the camera from the old pose to the new one, buffering one
//! command so quick taps are never dropped.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::pbr::DistanceFog;
use bevy::prelude::*;
use zrpg_core::{Command, Event, Facing, GridPos, Step, Turn};

use crate::atmosphere::Look;
use crate::auto::AutoMode;
use crate::input::{Action, Actions};
use crate::level::{GroundHeights, WALL_HEIGHT, cell_center, grid_dir};
use crate::screen::{LowResTarget, world_target};
use crate::{Frame, GameEvent, Run, publish};

const STEP_SECS: f32 = 0.16;
const TURN_SECS: f32 = 0.13;
const BUMP_SECS: f32 = 0.14;
const BUMP_DEPTH: f32 = 0.08;
/// How long an attack press is remembered while the swing is cooling down.
const ATTACK_BUFFER_SECS: f32 = 0.25;
const SHAKE_SECS: f32 = 0.25;
const SHAKE_SIZE: f32 = 0.04;
const DEATH_SECS: f32 = 0.7;
/// Fade to black while sinking down the stairs, then fade the new floor in.
const DESCEND_SECS: f32 = 0.45;
const ARRIVE_SECS: f32 = 0.4;
pub const EYE_HEIGHT: f32 = WALL_HEIGHT * 0.5;
/// Stand a little behind the cell centre so the side walls of the current
/// cell stay in view, as in the classics.
const EYE_BACK: f32 = 0.25;
const FOV_DEGREES: f32 = 60.0;

pub struct ViewPlugin;

impl Plugin for ViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera)
            .init_resource::<ScreenFade>()
            .add_systems(Update, drive_player.in_set(Frame::Command))
            .add_systems(
                Update,
                (react_to_events, animate_camera)
                    .chain()
                    .in_set(Frame::Present),
            );
    }
}

#[derive(Component)]
pub struct PlayerCamera;

/// How black the screen is (0 = clear, 1 = black), for floor transitions.
#[derive(Resource, Default)]
pub struct ScreenFade(pub f32);

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
    attack_buffer: f32,
    /// Remaining hit-shake time.
    shake: f32,
    /// Seconds since death, if dead.
    dead_for: Option<f32>,
    /// Seconds into the walk down the stairs, if taking them.
    descending: Option<f32>,
    /// Seconds since this floor (or run) began.
    arrived_for: f32,
    /// Which run/floor `rest` belongs to; a mismatch snaps the camera.
    synced_to: (u64, u32),
}

impl Animator {
    fn new(rest: Pose, synced_to: (u64, u32)) -> Self {
        Self {
            motion: Motion::Idle,
            elapsed: 0.0,
            duration: 1.0,
            rest,
            queued: None,
            attack_buffer: 0.0,
            shake: 0.0,
            dead_for: None,
            descending: None,
            arrived_for: 0.0,
            synced_to,
        }
    }

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
                pos: base.pos + dir * BUMP_DEPTH * (t * PI).sin(),
                yaw: base.yaw,
            },
        }
    }

    fn fade(&self) -> f32 {
        let out = self.descending.map_or(0.0, |d| d / DESCEND_SECS);
        let arrive = 1.0 - self.arrived_for / ARRIVE_SECS;
        out.max(arrive).clamp(0.0, 1.0)
    }

    /// The camera, `eye_height` above the ground at the pose.
    fn transform(&self, time: f32, eye_height: f32) -> Transform {
        let pose = self.pose();
        let yaw = Quat::from_rotation_y(pose.yaw);
        let mut eye = pose.pos + yaw * Vec3::Z * EYE_BACK + Vec3::Y * eye_height;
        let mut rot = yaw;
        if self.shake > 0.0 {
            let k = SHAKE_SIZE * self.shake / SHAKE_SECS;
            eye += Vec3::new((time * 71.0).sin(), (time * 53.0).cos(), 0.0) * k;
        }
        // Dying: slump to the floor and keel over sideways.
        if let Some(d) = self.dead_for {
            let e = ease((d / DEATH_SECS).min(1.0));
            eye.y -= eye_height * 0.7 * e;
            rot *= Quat::from_rotation_z(0.5 * e) * Quat::from_rotation_x(-0.25 * e);
        }
        // Descending: sink and look down the steps.
        if let Some(d) = self.descending {
            let e = ease((d / DESCEND_SECS).min(1.0));
            eye.y -= eye_height * 0.6 * e;
            rot *= Quat::from_rotation_x(-0.4 * e);
        }
        Transform::from_translation(eye).with_rotation(rot)
    }
}

fn ease(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn spawn_camera(mut commands: Commands, target: Res<LowResTarget>, run: Res<Run>) {
    let look = Look::of(run.floor.place);
    let anim = Animator::new(
        Pose::at(run.player.pos, run.player.facing),
        (run.run_seed, run.depth),
    );
    commands.spawn((
        PlayerCamera,
        Camera3d::default(),
        Camera {
            order: 0,
            clear_color: ClearColorConfig::Custom(look.sky),
            ..default()
        },
        world_target(&target),
        Projection::Perspective(PerspectiveProjection {
            fov: FOV_DEGREES.to_radians(),
            near: 0.05,
            far: look.view_distance,
            ..default()
        }),
        // Compress torch highlights instead of clipping them (needs no LUTs).
        Tonemapping::ReinhardLuminance,
        Msaa::Off,
        // Torchlit interiors: lights are tuned for indoor exposure.
        bevy::camera::Exposure { ev100: 6.4 },
        AmbientLight {
            color: look.ambient_color,
            brightness: look.ambient_brightness,
            ..default()
        },
        DistanceFog {
            color: look.fog_color(),
            falloff: look.fog,
            ..default()
        },
        anim.transform(0.0, look.eye_height),
        anim,
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
        Action::Attack | Action::Restart | Action::Auto | Action::Mute => return None,
    })
}

fn drive_player(
    time: Res<Time>,
    actions: Res<Actions>,
    auto: Res<AutoMode>,
    mut run: ResMut<Run>,
    mut cams: Query<&mut Animator, With<PlayerCamera>>,
    mut writer: MessageWriter<GameEvent>,
    mut buf: Local<Vec<Event>>,
) {
    let Ok(mut anim) = cams.single_mut() else {
        return;
    };
    let dt = time.delta_secs();

    // A new floor or a restart: snap to the new start, forget pending input.
    let floor_id = (run.run_seed, run.depth);
    if anim.synced_to != floor_id {
        *anim = Animator::new(Pose::at(run.player.pos, run.player.facing), floor_id);
        return;
    }
    if !run.player.is_alive() {
        anim.descending = None;
        return;
    }

    anim.elapsed += dt;
    if anim.busy() && anim.elapsed >= anim.duration {
        anim.motion = Motion::Idle;
    }

    // On the stairs: input is ignored until the fade completes, then down.
    if let Some(d) = anim.descending.as_mut() {
        *d += dt;
        if *d >= DESCEND_SECS {
            run.apply(Command::Descend, &mut buf);
            publish(&mut buf, &mut writer);
        }
        return;
    }

    // In auto mode the bot decides, through the same paths as a player.
    let bot = if auto.0 {
        zrpg_core::auto::next_command(&run)
    } else {
        None
    };

    // Attacks don't wait for movement: swinging mid-step feels right.
    // A press is remembered briefly; holding the button keeps swinging.
    if actions.just_pressed(Action::Attack) {
        anim.attack_buffer = ATTACK_BUFFER_SECS;
    }
    anim.attack_buffer = (anim.attack_buffer - dt).max(0.0);
    let ready = run.player.attack_cooldown <= 0.0;
    if ready
        && (anim.attack_buffer > 0.0
            || (actions.pressed(Action::Attack) && run.player.can_attack())
            || bot == Some(Command::Attack))
    {
        anim.attack_buffer = 0.0;
        run.apply(Command::Attack, &mut buf);
    }

    // Fresh movement presses are buffered (latest wins) so taps during
    // motion count.
    if let Some(cmd) = Action::ALL
        .into_iter()
        .filter(|a| actions.just_pressed(*a))
        .find_map(command_for)
    {
        anim.queued = Some(cmd);
    }

    if !anim.busy() {
        // Holding a direction keeps walking.
        let next = anim
            .queued
            .take()
            .map(|c| (c, false))
            .or_else(|| {
                Action::ALL
                    .into_iter()
                    .filter(|a| actions.pressed(*a))
                    .find_map(command_for)
                    .map(|c| (c, true))
            })
            .or_else(|| {
                bot.filter(|c| matches!(c, Command::Step(_) | Command::Turn(_)))
                    .map(|c| (c, false))
            });
        if let Some((cmd, from_hold)) = next {
            let first = buf.len();
            run.apply(cmd, &mut buf);
            start_motion(&mut anim, &buf[first..], from_hold);
            // Stepping onto the stairs takes them; no extra button needed.
            if run.on_stairs() {
                anim.descending = Some(0.0);
            }
        }
    }
    publish(&mut buf, &mut writer);
}

fn start_motion(anim: &mut Animator, events: &[Event], from_hold: bool) {
    let from = anim.rest;
    for event in events {
        match *event {
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
            _ => {}
        }
    }
}

fn react_to_events(
    time: Res<Time>,
    mut events: MessageReader<GameEvent>,
    mut cams: Query<&mut Animator, With<PlayerCamera>>,
) {
    let Ok(mut anim) = cams.single_mut() else {
        return;
    };
    let dt = time.delta_secs();
    anim.shake = (anim.shake - dt).max(0.0);
    anim.arrived_for += dt;
    if let Some(d) = anim.dead_for.as_mut() {
        *d += dt;
    }
    for GameEvent(event) in events.read() {
        match event {
            Event::EnemyStruck { damage, .. } if *damage > 0 => anim.shake = SHAKE_SECS,
            Event::PlayerDied => anim.dead_for = Some(0.0),
            _ => {}
        }
    }
}

fn animate_camera(
    time: Res<Time>,
    run: Res<Run>,
    ground: Res<GroundHeights>,
    mut fade: ResMut<ScreenFade>,
    mut cams: Query<(&Animator, &mut Transform), With<PlayerCamera>>,
) {
    for (anim, mut t) in &mut cams {
        let eye_height = Look::of(run.floor.place).eye_height;
        *t = anim.transform(time.elapsed_secs(), eye_height);
        // Wade down into water and climb stairs smoothly, never in a jump.
        t.translation.y += ground.at(anim.pose().pos);
        fade.0 = anim.fade();
    }
}
