//! Light. The player carries a torch, and some walls hold torches in
//! sconces; all of them flicker. Every surface has a normal map and a
//! roughness map, so as the light moves, it rakes across block edges, joints
//! and carvings, and glints on worn floor tiles and damp moss.
//!
//! Everything else stays cheap: no shadow maps, a handful of point lights per
//! floor, and a dim warm ambient so unlit corners never go fully black.

use bevy::prelude::*;

use crate::level::Torch;
use crate::textures::{self, env};
use crate::view::PlayerCamera;
use crate::{Frame, Run};

const TORCH_COLOR: Color = Color::srgb(1.0, 0.84, 0.64);
/// The player's torch: held up, to the right and a little ahead, so light
/// rakes across what is in front rather than hitting it flat.
const HAND_TORCH_OFFSET: Vec3 = Vec3::new(0.28, 0.1, -0.25);
const HAND_TORCH_LUMENS: f32 = 11_000.0;
const HAND_TORCH_RANGE: f32 = 6.0;
const WALL_TORCH_LUMENS: f32 = 6_000.0;
const WALL_TORCH_RANGE: f32 = 4.5;
/// The light sits a little in front of the flame so its own wall is lit at
/// a grazing angle, which shows the stonework best.
const WALL_TORCH_STANDOFF: f32 = 0.15;
/// Cool shadows against warm torchlight.
pub const AMBIENT_COLOR: Color = Color::srgb(0.72, 0.78, 0.88);
pub const AMBIENT_BRIGHTNESS: f32 = 60.0;
const FLAME_SIZE: Vec2 = Vec2::new(0.11, 0.17);

pub struct LightingPlugin;

impl Plugin for LightingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_assets)
            .add_systems(PostStartup, light_hand_torch)
            .add_systems(Update, (flicker, animate_flames).in_set(Frame::Present));
    }
}

/// Shared mesh and material for torch flames.
#[derive(Resource)]
pub struct TorchAssets {
    flame_mesh: Handle<Mesh>,
    flame_material: Handle<StandardMaterial>,
}

/// Wobbles a light's intensity (and a flame's height) around `base`.
#[derive(Component)]
struct Flicker {
    base: f32,
    phase: f32,
}

impl Flicker {
    /// A few out-of-step sines read as fire, without any randomness.
    fn factor(&self, t: f32) -> f32 {
        let p = self.phase;
        1.0 + 0.07 * (t * 7.3 + p).sin()
            + 0.05 * (t * 13.1 + p * 1.7).sin()
            + 0.03 * (t * 23.9 + p * 2.9).sin()
    }
}

#[derive(Component)]
struct Flame {
    phase: f32,
}

fn load_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(TorchAssets {
        flame_mesh: meshes.add(Rectangle::new(FLAME_SIZE.x, FLAME_SIZE.y)),
        // Fire makes its own light: unlit, and it shines through the fog
        // like a beacon at the end of a corridor.
        flame_material: materials.add(StandardMaterial {
            base_color_texture: Some(images.add(textures::embedded(env::FLAME))),
            unlit: true,
            fog_enabled: false,
            alpha_mode: AlphaMode::Mask(0.5),
            cull_mode: None,
            ..default()
        }),
    });
}

fn light_hand_torch(mut commands: Commands, cams: Query<Entity, With<PlayerCamera>>) {
    for cam in &cams {
        commands.entity(cam).with_child((
            PointLight {
                color: TORCH_COLOR,
                intensity: HAND_TORCH_LUMENS,
                range: HAND_TORCH_RANGE,
                radius: 0.05,
                shadow_maps_enabled: false,
                ..default()
            },
            Flicker {
                base: HAND_TORCH_LUMENS,
                phase: 0.0,
            },
            Transform::from_translation(HAND_TORCH_OFFSET),
        ));
    }
}

/// A wall torch's flame and light, as children of the level.
pub fn spawn_wall_torch(
    parent: &mut ChildSpawnerCommands,
    assets: &TorchAssets,
    torch: &Torch,
    phase: f32,
) {
    parent.spawn((
        Flame { phase },
        Mesh3d(assets.flame_mesh.clone()),
        MeshMaterial3d(assets.flame_material.clone()),
        Transform::from_translation(torch.flame),
    ));
    parent.spawn((
        PointLight {
            color: TORCH_COLOR,
            intensity: WALL_TORCH_LUMENS,
            range: WALL_TORCH_RANGE,
            radius: 0.05,
            shadow_maps_enabled: false,
            ..default()
        },
        Flicker {
            base: WALL_TORCH_LUMENS,
            phase,
        },
        Transform::from_translation(torch.flame + torch.facing * WALL_TORCH_STANDOFF),
    ));
}

fn flicker(time: Res<Time>, run: Res<Run>, mut lights: Query<(&Flicker, &mut PointLight)>) {
    let t = time.elapsed_secs();
    // The dead see no torchlight.
    let fade = if run.player.is_alive() { 1.0 } else { 0.35 };
    for (f, mut light) in &mut lights {
        light.intensity = f.base * f.factor(t) * fade;
    }
}

/// Flames face the camera (like the enemies) and dance.
fn animate_flames(
    time: Res<Time>,
    cams: Query<&Transform, (With<PlayerCamera>, Without<Flame>)>,
    mut flames: Query<(&Flame, &mut Transform)>,
) {
    let Ok(cam) = cams.single() else {
        return;
    };
    let (yaw, _, _) = cam.rotation.to_euler(EulerRot::YXZ);
    let t = time.elapsed_secs();
    for (flame, mut tf) in &mut flames {
        let f = Flicker {
            base: 1.0,
            phase: flame.phase,
        };
        let stretch = f.factor(t * 1.3);
        tf.rotation = Quat::from_rotation_y(yaw)
            * Quat::from_rotation_z(0.06 * (t * 5.0 + flame.phase).sin());
        tf.scale = Vec3::new(2.0 - stretch, stretch, 1.0);
    }
}
