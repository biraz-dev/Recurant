//! Billboard enemies: flat sprites standing in the 3D dungeon, always turned
//! to face the camera. Movement, wind-ups, hits and deaths come from core
//! events and are eased here.

use std::f32::consts::PI;

use bevy::prelude::*;
use zrpg_core::{EnemyId, Event};

use crate::level::cell_center;
use crate::view::PlayerCamera;
use crate::{Frame, GameEvent, Run, textures};

const SPRITE_SIZE: f32 = 0.7;
/// Slightly longer than the core's step interval would allow, so motion
/// reads as a deliberate shuffle rather than a teleport.
const STEP_ANIM_SECS: f32 = 0.3;
const FLASH_SECS: f32 = 0.2;
const LUNGE_SECS: f32 = 0.2;
const DIE_SECS: f32 = 0.45;

pub struct EnemiesPlugin;

impl Plugin for EnemiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_assets).add_systems(
            Update,
            (sync_roster, react_to_events, animate)
                .chain()
                .in_set(Frame::Present),
        );
    }
}

#[derive(Resource)]
struct EnemyAssets {
    mesh: Handle<Mesh>,
    texture: Handle<Image>,
}

#[derive(Component)]
struct EnemySprite {
    id: EnemyId,
    /// Ground position, eased between cells.
    from: Vec3,
    to: Vec3,
    move_t: f32,
    /// 0..1, how far into the wind-up pose (eased each frame).
    wind: f32,
    flash: f32,
    lunge: f32,
    dying: Option<f32>,
    material: Handle<StandardMaterial>,
}

impl EnemySprite {
    fn ground(&self) -> Vec3 {
        let t = (self.move_t / STEP_ANIM_SECS).min(1.0);
        self.from.lerp(self.to, t * t * (3.0 - 2.0 * t))
    }
}

/// Which run/floor the spawned sprites belong to.
#[derive(Default, PartialEq)]
struct Roster(Option<(u64, u32)>);

fn load_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
) {
    commands.insert_resource(EnemyAssets {
        mesh: meshes.add(Rectangle::new(SPRITE_SIZE, SPRITE_SIZE)),
        texture: images.add(textures::slime()),
    });
}

fn sync_roster(
    mut commands: Commands,
    run: Res<Run>,
    assets: Res<EnemyAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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
    for enemy in &run.enemies {
        // One material each so hit flashes tint only the enemy that was hit.
        let material = materials.add(StandardMaterial {
            base_color_texture: Some(assets.texture.clone()),
            alpha_mode: AlphaMode::Mask(0.5),
            unlit: true,
            cull_mode: None,
            ..default()
        });
        let at = cell_center(enemy.pos);
        commands.spawn((
            EnemySprite {
                id: enemy.id,
                from: at,
                to: at,
                move_t: STEP_ANIM_SECS,
                wind: 0.0,
                flash: 0.0,
                lunge: 0.0,
                dying: None,
                material: material.clone(),
            },
            Mesh3d(assets.mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_translation(at),
        ));
    }
}

fn react_to_events(mut events: MessageReader<GameEvent>, mut sprites: Query<&mut EnemySprite>) {
    for GameEvent(event) in events.read() {
        let id = match *event {
            Event::EnemyMoved { id, .. }
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
            }
            Event::EnemyHurt { .. } => s.flash = FLASH_SECS,
            Event::EnemyStruck { .. } => s.lunge = LUNGE_SECS,
            Event::EnemyDied { .. } => s.dying = Some(0.0),
            _ => {}
        }
    }
}

fn animate(
    mut commands: Commands,
    time: Res<Time>,
    run: Res<Run>,
    cams: Query<&Transform, (With<PlayerCamera>, Without<EnemySprite>)>,
    mut sprites: Query<(Entity, &mut EnemySprite, &mut Transform)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(cam) = cams.single() else {
        return;
    };
    let dt = time.delta_secs();
    let now = time.elapsed_secs();
    // Face the camera's heading (ignoring pitch/roll) so sprites stay upright.
    let (cam_yaw, _, _) = cam.rotation.to_euler(EulerRot::YXZ);
    let facing = Quat::from_rotation_y(cam_yaw);
    let player = cell_center(run.player.pos);

    for (entity, mut s, mut t) in &mut sprites {
        s.move_t += dt;
        s.flash = (s.flash - dt).max(0.0);
        s.lunge = (s.lunge - dt).max(0.0);
        let winding = run
            .enemies
            .iter()
            .any(|e| e.id == s.id && e.is_winding_up());
        let target = if winding { 1.0 } else { 0.0 };
        s.wind += (target - s.wind) * (dt * 10.0).min(1.0);

        let ground = s.ground();
        let toward = (player - ground).normalize_or_zero();
        let lunge = (s.lunge / LUNGE_SECS * PI).sin().max(0.0);
        // Idle squish, wind-up swell, strike lunge.
        let phase = now * 4.0 + s.id as f32 * 1.7;
        let mut sy = 1.0 + 0.05 * phase.sin() + 0.15 * s.wind;
        let mut sx = 1.0 - 0.04 * phase.sin() + 0.1 * s.wind;
        let mut pos = ground + toward * (0.08 * s.wind + 0.3 * lunge);

        if let Some(d) = s.dying.as_mut() {
            *d += dt;
            let k = (*d / DIE_SECS).min(1.0);
            sy *= 1.0 - k;
            sx *= 1.0 + 0.6 * k;
            if k >= 1.0 {
                commands.entity(entity).despawn();
                continue;
            }
        }
        pos.y = SPRITE_SIZE * 0.5 * sy;
        *t = Transform::from_translation(pos)
            .with_rotation(facing)
            .with_scale(Vec3::new(sx, sy, 1.0));

        // Tint: red on hit, hot orange while winding up.
        let color = if s.flash > 0.0 {
            Color::srgb(1.0, 0.35, 0.3)
        } else {
            Color::srgb(1.0, 1.0 - 0.25 * s.wind, 1.0 - 0.55 * s.wind)
        };
        if let Some(mut m) = materials.get_mut(&s.material)
            && m.base_color != color
        {
            m.base_color = color;
        }
    }
}
