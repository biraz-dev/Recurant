//! Billboard enemies: flat sprites standing in the 3D dungeon, always turned
//! to face the camera. Movement, wind-ups, hits and deaths come from core
//! events and are eased here.
//!
//! The zombie is a single still, brought to life by sway and lurch. The
//! creature kits (`art/kits`, packed by `tools/make_creatures.py` into
//! `sheets.rs`) are animated: each has four views (front, back, left, right)
//! of a locomotion cycle and three attacks, six frames each. Which view
//! shows depends on where the creature is heading relative to the camera,
//! as in the classic sprite shooters.
//!
//! The kits' animations are generated prototypes, and their documentation
//! says so; this code leans on them only where they are safe:
//!
//! - Gameplay timing stays in `zrpg_core`. An attack's lead-in frames are
//!   stretched over the core's wind-up, so the kit's hit frame appears
//!   exactly when the core deals the damage; its suggested timings never
//!   decide anything.
//! - Idle is walk frame 0, as the kits suggest; a stopping creature finishes
//!   its stride first rather than snapping to it.
//! - The kits have no death animation: the death collapse is drawn here.
//! - Left and right are separate drawings, never mirrored.

use std::f32::consts::PI;

use bevy::light::NotShadowCaster;
use bevy::math::Affine2;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use zrpg_core::{Enemy, EnemyId, EnemyKind, EnemyState, Event};

use crate::atmosphere::{GradedMaterial, Look};
use crate::level::{GroundHeights, cell_center};
use crate::view::PlayerCamera;
use crate::{Frame, GameEvent, Run, sheets, textures};

/// The zombie, made from `art/zombie-reference.png` by `tools/make_sprite.py`.
/// Embedded so no platform has to load files at runtime.
const ZOMBIE_PNG: &[u8] = include_bytes!("../assets/zombie.png");
/// Standing height in world units (walls are 1.0).
const SPRITE_HEIGHT: f32 = 0.95;
/// Slightly longer than the core's step interval would allow, so motion
/// reads as a deliberate shuffle rather than a teleport.
const STEP_ANIM_SECS: f32 = 0.3;
const FLASH_SECS: f32 = 0.2;
const LUNGE_SECS: f32 = 0.2;
/// Dying: like an apparition in a dream, the body pales, drifts up and
/// comes apart pixel by pixel over this long.
const DISSOLVE_SECS: f32 = 1.1;
/// How far it drifts up and stretches as it goes.
const DISSOLVE_RISE: f32 = 0.22;
const DISSOLVE_STRETCH: f32 = 0.18;
/// The pale it turns as it goes.
const DREAM_PALE: Color = Color::srgb(0.75, 0.85, 1.0);

/// Sheet layout, shared by every creature kit: 6 frames across, and for each
/// of the four views, four rows (locomotion, then three attacks).
const CELL_PX: f32 = 128.0;
const FRAMES: usize = 6;
const ACTIONS: usize = 4;
const ROWS: usize = 4 * ACTIONS;
/// The kits' ground pivot in a 128 px cell, from its top-left.
const PIVOT_PX: Vec2 = Vec2::new(64.0, 116.0);
/// How much better another view must fit before the sprite switches to it,
/// so a creature seen at 45 degrees doesn't flicker between two.
const VIEW_HYSTERESIS: f32 = 0.15;
/// Hovering creatures float this far up, bobbing gently.
const HOVER_HEIGHT: f32 = 0.06;

/// One action on a sheet: a row of six frames.
pub struct Clip {
    pub name: &'static str,
    pub fps: f32,
    /// The frame (zero-based) the blow lands on; attacks only.
    pub hit: Option<usize>,
}

/// A creature kit's sprite sheet (see `tools/make_creatures.py`).
pub struct Sheet {
    pub png: &'static [u8],
    /// Standing height in world units.
    pub height: f32,
    /// Standing body height in sheet pixels (front walk, frame 0).
    pub body_px: f32,
    pub locomotion: Clip,
    pub attacks: [Clip; 3],
}

impl Sheet {
    /// Wraiths hover: their cycle never stops.
    fn hovers(&self) -> bool {
        self.locomotion.name == "hover"
    }
}

pub struct EnemiesPlugin;

impl Plugin for EnemiesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EnemyAssets>().add_systems(
            Update,
            (sync_roster, react_to_events, animate)
                .chain()
                .in_set(Frame::Present),
        );
    }
}

/// A kind's mesh and texture, decoded when a floor needs them.
struct KindArt {
    mesh: Handle<Mesh>,
    texture: Handle<Image>,
    sheet: Option<&'static Sheet>,
}

#[derive(Resource, Default)]
struct EnemyAssets {
    kinds: HashMap<EnemyKind, KindArt>,
}

impl EnemyAssets {
    fn art(
        &mut self,
        kind: EnemyKind,
        meshes: &mut Assets<Mesh>,
        images: &mut Assets<Image>,
    ) -> &KindArt {
        self.kinds.entry(kind).or_insert_with(|| {
            let sheet = sheets::sheet(kind);
            let image = textures::embedded(sheet.map_or(ZOMBIE_PNG, |s| s.png));
            // Every mesh has its origin at the creature's feet.
            let mesh = match sheet {
                Some(s) => {
                    let unit = s.height / s.body_px;
                    let above = PIVOT_PX.y * unit;
                    let size = CELL_PX * unit;
                    Mesh::from(Rectangle::new(size, size))
                        .translated_by(Vec3::Y * (above - size * 0.5))
                }
                None => {
                    let aspect = image.width() as f32 / image.height() as f32;
                    Mesh::from(Rectangle::new(SPRITE_HEIGHT * aspect, SPRITE_HEIGHT))
                        .translated_by(Vec3::Y * SPRITE_HEIGHT * 0.5)
                }
            };
            KindArt {
                mesh: meshes.add(mesh),
                texture: images.add(image),
                sheet,
            }
        })
    }
}

/// Which way a sprite is seen from: its sheet's views, in row order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum View {
    Front,
    Back,
    Left,
    Right,
}

/// An attack in progress: which of the three, and how long since the blow
/// landed (`None` while still winding up).
#[derive(Clone, Copy, Debug)]
struct Attack {
    clip: usize,
    since_strike: Option<f32>,
}

/// A death in progress: seconds since it began.
#[derive(Clone, Copy, Debug)]
struct Dying {
    t: f32,
}

impl Dying {
    /// 0 whole, 1 gone.
    fn progress(&self) -> f32 {
        (self.t / DISSOLVE_SECS).min(1.0)
    }
}

#[derive(Component)]
struct EnemySprite {
    id: EnemyId,
    sheet: Option<&'static Sheet>,
    /// Ground position, eased between cells.
    from: Vec3,
    to: Vec3,
    move_t: f32,
    /// Which way it is going or looking, on the ground plane.
    heading: Vec3,
    view: View,
    /// Seconds of locomotion played.
    cycle_t: f32,
    attack: Option<Attack>,
    attacks_made: u32,
    /// 0..1, how far into the wind-up pose (eased each frame).
    wind: f32,
    flash: f32,
    lunge: f32,
    dying: Option<Dying>,
    /// Steps taken so far; alternates the lurch from side to side.
    steps: u32,
    material: Handle<GradedMaterial>,
}

impl EnemySprite {
    fn ground(&self) -> Vec3 {
        let t = (self.move_t / STEP_ANIM_SECS).min(1.0);
        self.from.lerp(self.to, t * t * (3.0 - 2.0 * t))
    }

    /// The sheet cell to show now: (frame, row).
    fn cell(&self, sheet: &Sheet, enemy: Option<&Enemy>) -> (usize, usize) {
        let (action, frame) = match self.attack {
            Some(Attack { clip, since_strike }) => {
                let c = &sheet.attacks[clip];
                let hit = c.hit.unwrap_or(FRAMES / 2).clamp(1, FRAMES - 1);
                let frame = match since_strike {
                    // Lead-in frames spread over the core's wind-up.
                    None => {
                        let progress = enemy.map_or(0.0, |e| match e.state {
                            EnemyState::WindUp { left } => 1.0 - left / e.stats().wind_up,
                            _ => 1.0,
                        });
                        ((progress * hit as f32) as usize).min(hit - 1)
                    }
                    Some(t) => (hit + (t * c.fps) as usize).min(FRAMES - 1),
                };
                (1 + clip, frame)
            }
            None => (0, cycle_frame(sheet, self.cycle_t)),
        };
        (frame, self.view as usize * ACTIONS + action)
    }
}

fn cycle_frame(sheet: &Sheet, t: f32) -> usize {
    (t * sheet.locomotion.fps) as usize % FRAMES
}

/// Which run/floor the spawned sprites belong to.
#[derive(Default, PartialEq)]
struct Roster(Option<(u64, u32)>);

#[allow(clippy::too_many_arguments)]
fn sync_roster(
    mut commands: Commands,
    run: Res<Run>,
    mut assets: ResMut<EnemyAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<GradedMaterial>>,
    sprites: Query<Entity, With<EnemySprite>>,
    mut roster: Local<Roster>,
) {
    let id = Some((run.run_seed, run.depth));
    if roster.0 == id {
        return;
    }
    roster.0 = id;
    for e in &sprites {
        commands.entity(e).despawn();
    }
    let look = Look::of(&run.floor);
    let player = cell_center(run.player.pos);
    for enemy in &run.enemies {
        let art = assets.art(enemy.kind, &mut meshes, &mut images);
        // One material each so hit flashes tint only the enemy that was hit.
        let material = materials.add(look.graded(StandardMaterial {
            base_color_texture: Some(art.texture.clone()),
            alpha_mode: AlphaMode::Mask(0.5),
            // Lit like the walls: enemies loom out of the dark as torchlight
            // finds them. The wind-up and hit tells also glow (emissive), so
            // they read even outside the light.
            reflectance: 0.2,
            perceptual_roughness: 0.9,
            cull_mode: None,
            uv_transform: cell_transform(0, 0),
            ..default()
        }));
        let at = cell_center(enemy.pos);
        commands.spawn((
            EnemySprite {
                id: enemy.id,
                sheet: art.sheet,
                from: at,
                to: at,
                move_t: STEP_ANIM_SECS,
                // Sentries stand watching the way the player comes in.
                heading: (player - at).normalize_or(Vec3::Z),
                view: View::Front,
                cycle_t: 0.0,
                attack: None,
                attacks_made: 0,
                wind: 0.0,
                flash: 0.0,
                lunge: 0.0,
                dying: None,
                steps: 0,
                material: material.clone(),
            },
            Mesh3d(art.mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_translation(at),
            // A billboard's shadow would swing with the camera.
            NotShadowCaster,
        ));
    }
    // Keep only this floor's sheets decoded: each is a few megabytes of
    // texture, which matters on phones and in the browser.
    assets
        .kinds
        .retain(|kind, _| run.enemies.iter().any(|e| e.kind == *kind));
}

/// UV transform selecting one cell of a sheet (or the whole of a still).
fn cell_transform(frame: usize, row: usize) -> Affine2 {
    Affine2::from_scale_angle_translation(
        Vec2::new(1.0 / FRAMES as f32, 1.0 / ROWS as f32),
        0.0,
        Vec2::new(frame as f32 / FRAMES as f32, row as f32 / ROWS as f32),
    )
}

fn react_to_events(
    mut events: MessageReader<GameEvent>,
    run: Res<Run>,
    mut sprites: Query<&mut EnemySprite>,
) {
    let player = cell_center(run.player.pos);
    for GameEvent(event) in events.read() {
        let id = match *event {
            Event::EnemyMoved { id, .. }
            | Event::EnemyWindUp { id }
            | Event::EnemyHurt { id, .. }
            | Event::EnemyStruck { id, .. }
            | Event::EnemyDied { id, .. } => id,
            _ => continue,
        };
        let Some(mut s) = sprites.iter_mut().find(|s| s.id == id) else {
            continue;
        };
        match *event {
            Event::EnemyMoved { to, .. } => {
                s.from = s.ground();
                s.to = cell_center(to);
                s.move_t = 0.0;
                s.steps += 1;
                s.heading = (s.to - s.from).normalize_or(s.heading);
            }
            Event::EnemyWindUp { .. } => {
                // Vary the blow: each wind-up draws one of the three.
                let clip =
                    (zrpg_core::rng::mix64(id as u64 ^ (s.attacks_made as u64) << 32) % 3) as usize;
                s.attacks_made += 1;
                s.attack = Some(Attack {
                    clip,
                    since_strike: None,
                });
                s.heading = (player - s.to).normalize_or(s.heading);
            }
            Event::EnemyHurt { staggered, .. } => {
                s.flash = FLASH_SECS;
                // A stagger knocks it out of its attack.
                if staggered {
                    s.attack = None;
                }
            }
            Event::EnemyStruck { .. } => {
                s.lunge = LUNGE_SECS;
                let clip = s.attack.map_or(0, |a| a.clip);
                s.attack = Some(Attack {
                    clip,
                    since_strike: Some(0.0),
                });
            }
            Event::EnemyDied { .. } => {
                s.dying = Some(Dying { t: 0.0 });
                s.attack = None;
            }
            _ => {}
        }
    }
}

/// The view that best fits a creature heading `heading`, seen from a camera
/// in direction `to_cam` with its right-hand side along `cam_right`.
fn pick_view(heading: Vec3, to_cam: Vec3, cam_right: Vec3, current: View) -> View {
    let fit = |v: View| match v {
        View::Front => heading.dot(to_cam),
        View::Back => -heading.dot(to_cam),
        // Facing screen-right: heading along the camera's right.
        View::Right => heading.dot(cam_right),
        View::Left => -heading.dot(cam_right),
    };
    let best = [View::Front, View::Back, View::Left, View::Right]
        .into_iter()
        .max_by(|a, b| fit(*a).total_cmp(&fit(*b)))
        .unwrap_or(current);
    if fit(best) > fit(current) + VIEW_HYSTERESIS {
        best
    } else {
        current
    }
}

#[allow(clippy::too_many_arguments)]
fn animate(
    mut commands: Commands,
    time: Res<Time>,
    run: Res<Run>,
    ground_heights: Res<GroundHeights>,
    cams: Query<&Transform, (With<PlayerCamera>, Without<EnemySprite>)>,
    mut sprites: Query<(Entity, &mut EnemySprite, &mut Transform)>,
    mut materials: ResMut<Assets<GradedMaterial>>,
) {
    let Ok(cam) = cams.single() else {
        return;
    };
    let dt = time.delta_secs();
    let now = time.elapsed_secs();
    // Face the camera's heading (ignoring pitch/roll) so sprites stay upright.
    let (cam_yaw, _, _) = cam.rotation.to_euler(EulerRot::YXZ);
    let facing = Quat::from_rotation_y(cam_yaw);
    let cam_right = facing * Vec3::X;
    let player = cell_center(run.player.pos);

    for (entity, mut s, mut t) in &mut sprites {
        s.move_t += dt;
        s.flash = (s.flash - dt).max(0.0);
        s.lunge = (s.lunge - dt).max(0.0);
        let enemy = run.enemies.iter().find(|e| e.id == s.id);
        let winding = enemy.is_some_and(|e| e.is_winding_up());
        let target = if winding { 1.0 } else { 0.0 };
        s.wind += (target - s.wind) * (dt * 10.0).min(1.0);

        let ground = s.ground();
        let toward = (player - ground).normalize_or_zero();
        let lunge = (s.lunge / LUNGE_SECS * PI).sin().max(0.0);
        let moving = s.move_t < STEP_ANIM_SECS;
        // Next to the player (and not walking off), it squares up to you.
        if enemy.is_some_and(|e| e.pos.manhattan(run.player.pos) == 1) && !moving {
            s.heading = toward;
        }

        let mut feet = ground + Vec3::Y * ground_heights.at(ground);
        let (roll, mut sx, mut sy);
        if let Some(sheet) = s.sheet {
            // The drawings carry the motion; the sprite only leans in.
            roll = 0.0;
            sx = 1.0;
            sy = 1.0;
            feet += toward * 0.1 * lunge;
            let to_cam = (cam.translation - feet).with_y(0.0).normalize_or_zero();
            s.view = pick_view(s.heading, to_cam, cam_right, s.view);
            if let Some(a) = s.attack.as_mut()
                && let Some(since) = a.since_strike.as_mut()
            {
                *since += dt;
                let clip = &sheet.attacks[a.clip];
                let hit = clip.hit.unwrap_or(FRAMES / 2);
                if (hit as f32 + *since * clip.fps) >= FRAMES as f32 {
                    s.attack = None;
                }
            }
            // Walk while moving, then finish the stride to frame 0 (idle).
            // Hoverers never stop.
            if sheet.hovers() {
                s.cycle_t += dt;
                feet.y += HOVER_HEIGHT * (1.0 + (now * 2.1 + s.id as f32).sin());
            } else if moving || cycle_frame(sheet, s.cycle_t) != 0 {
                s.cycle_t += dt;
            } else {
                s.cycle_t = 0.0;
            }
        } else {
            // A slow sway, a lurch to alternating sides on each step, rearing
            // up during the wind-up, and a lunge when the blow lands.
            let phase = now * 1.6 + s.id as f32 * 1.7;
            let step_t = (s.move_t / STEP_ANIM_SECS).min(1.0);
            let side = if s.steps % 2 == 0 { 1.0 } else { -1.0 };
            roll = 0.035 * phase.sin() + side * 0.09 * (step_t * PI).sin();
            sy = 1.0 + 0.012 * (phase * 2.0).sin() + 0.07 * s.wind;
            sx = 1.0 + 0.04 * s.wind;
            feet += toward * (0.06 * s.wind + 0.3 * lunge);
        }

        let mut rotation = facing * Quat::from_rotation_z(roll);
        let mut dissolve = 0.0;
        if let Some(d) = s.dying.as_mut() {
            d.t += dt;
            if d.t >= DISSOLVE_SECS {
                commands.entity(entity).despawn();
                continue;
            }
            // Drift up, thin out, and come apart.
            let k = d.progress();
            let ease = k * (2.0 - k);
            feet.y += DISSOLVE_RISE * ease;
            sy = 1.0 + DISSOLVE_STRETCH * ease;
            sx = 1.0 - 0.5 * DISSOLVE_STRETCH * ease;
            rotation = facing;
            dissolve = k.powf(0.8);
        }
        // Pivot at the feet so swaying never lifts them off the floor.
        *t = Transform::from_translation(feet)
            .with_rotation(rotation)
            .with_scale(Vec3::new(sx, sy, 1.0));

        // Tint and glow: red on hit, hot orange while winding up.
        let (color, glow) = if let Some(d) = s.dying {
            // Paling into an apparition.
            let k = d.progress();
            (
                Color::WHITE.mix(&DREAM_PALE, k),
                LinearRgba::rgb(0.05, 0.07, 0.1) * k,
            )
        } else if s.flash > 0.0 {
            (
                Color::srgb(1.0, 0.35, 0.3),
                LinearRgba::rgb(0.14, 0.02, 0.01),
            )
        } else {
            (
                Color::srgb(1.0, 1.0 - 0.25 * s.wind, 1.0 - 0.55 * s.wind),
                LinearRgba::rgb(0.11, 0.045, 0.005) * s.wind,
            )
        };
        let uv = match s.sheet {
            Some(sheet) => {
                let (frame, row) = s.cell(sheet, enemy);
                cell_transform(frame, row)
            }
            None => Affine2::IDENTITY,
        };
        // Sheets break up on their 128-pixel cells; the zombie still on its
        // own smaller grid.
        let grid = if s.sheet.is_some() { CELL_PX } else { 64.0 };
        if let Some(mut m) = materials.get_mut(&s.material)
            && (m.base.base_color != color
                || m.base.emissive != glow
                || m.base.uv_transform != uv
                || m.extension.dissolve() != dissolve)
        {
            m.base.base_color = color;
            m.base.emissive = glow;
            m.base.uv_transform = uv;
            m.extension.set_dissolve(dissolve, grid);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn views_follow_heading_relative_to_the_camera() {
        // Camera looking north (-Z), right is +X; the creature is north of it.
        let (to_cam, right) = (Vec3::Z, Vec3::X);
        let v = |heading: Vec3| pick_view(heading, to_cam, right, View::Front);
        assert_eq!(v(Vec3::Z), View::Front);
        assert_eq!(v(-Vec3::Z), View::Back);
        assert_eq!(v(Vec3::X), View::Right);
        assert_eq!(v(-Vec3::X), View::Left);
        // Near a diagonal, the current view holds.
        let diagonal = Vec3::new(1.0, 0.0, 1.05).normalize();
        assert_eq!(pick_view(diagonal, to_cam, right, View::Right), View::Right);
        assert_eq!(pick_view(diagonal, to_cam, right, View::Front), View::Front);
    }

    #[test]
    fn attack_frames_land_on_the_cores_blow() {
        let sheet = &sheets::UNDEAD;
        let mut enemy = Enemy::of_kind(0, zrpg_core::GridPos::new(1, 1), EnemyKind::Undead);
        let wind_up = enemy.stats().wind_up;
        let mut s = EnemySprite {
            id: 0,
            sheet: Some(sheet),
            from: Vec3::ZERO,
            to: Vec3::ZERO,
            move_t: 1.0,
            heading: Vec3::Z,
            view: View::Front,
            cycle_t: 0.0,
            attack: Some(Attack {
                clip: 2,
                since_strike: None,
            }),
            attacks_made: 1,
            wind: 0.0,
            flash: 0.0,
            lunge: 0.0,
            dying: None,
            steps: 0,
            material: Handle::default(),
        };
        let hit = sheet.attacks[2].hit.unwrap();
        // Throughout the wind-up, only lead-in frames show, in order.
        let mut last = 0;
        for k in 0..=10 {
            enemy.state = EnemyState::WindUp {
                left: wind_up * (1.0 - k as f32 / 10.0).max(0.001),
            };
            let (frame, row) = s.cell(sheet, Some(&enemy));
            assert!(frame < hit && frame >= last, "frame {frame} at {k}/10");
            assert_eq!(row, 3, "front view, third attack");
            last = frame;
        }
        // The blow shows the kit's hit frame, then the follow-through.
        s.attack.as_mut().unwrap().since_strike = Some(0.0);
        assert_eq!(s.cell(sheet, Some(&enemy)).0, hit);
        s.attack.as_mut().unwrap().since_strike = Some(10.0);
        assert_eq!(s.cell(sheet, Some(&enemy)).0, FRAMES - 1);
    }

    #[test]
    fn sheet_cells_are_exact() {
        let t = cell_transform(5, 15);
        let (lo, hi) = (
            t.transform_point2(Vec2::ZERO),
            t.transform_point2(Vec2::ONE),
        );
        assert!((lo - Vec2::new(5.0 / 6.0, 15.0 / 16.0)).length() < 1e-6);
        assert!((hi - Vec2::ONE).length() < 1e-6);
    }
}
