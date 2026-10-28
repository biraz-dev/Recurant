//! Gold on the floor: a little heap of coins wherever a kill dropped some,
//! turning to face you and catching the light, until you walk over it.
//! The coins are drawn here, pixel by pixel; no art file needed.

use bevy::asset::RenderAssetUsages;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use zrpg_core::GridPos;

use crate::atmosphere::{GradedMaterial, Look};
use crate::level::{GroundHeights, cell_center};
use crate::view::PlayerCamera;
use crate::{Frame, Run};

/// The heap's size in world units (a cell is 1.0).
const HEAP_SIZE: Vec2 = Vec2::new(0.3, 0.2);
/// Sprite pixels.
const W: u32 = 18;
const H: u32 = 12;
/// A soft glow, so gold catches the eye even in the dark.
const GLINT: LinearRgba = LinearRgba::rgb(0.09, 0.06, 0.01);

pub struct LootPlugin;

impl Plugin for LootPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load).add_systems(
            Update,
            (sync, turn_to_camera).chain().in_set(Frame::Present),
        );
    }
}

#[derive(Resource)]
struct LootAssets {
    mesh: Handle<Mesh>,
    texture: Handle<Image>,
}

#[derive(Component)]
struct Heap(GridPos);

fn load(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
) {
    let mesh = Mesh::from(Rectangle::new(HEAP_SIZE.x, HEAP_SIZE.y))
        .translated_by(Vec3::Y * HEAP_SIZE.y * 0.5);
    commands.insert_resource(LootAssets {
        mesh: meshes.add(mesh),
        texture: images.add(heap_image()),
    });
}

/// A heap of coins: overlapping gold discs with a dark rim and a bright
/// highlight, the front ones lowest.
fn heap_image() -> Image {
    // (centre x, centre y, radius) in pixels; drawn back to front.
    const COINS: [(f32, f32, f32); 6] = [
        (6.0, 6.0, 3.2),
        (11.5, 5.5, 3.2),
        (9.0, 3.5, 3.0),
        (4.0, 8.5, 3.2),
        (9.0, 8.5, 3.4),
        (14.0, 8.8, 3.0),
    ];
    let mut data = vec![0u8; (W * H * 4) as usize];
    for (cx, cy, r) in COINS {
        for y in 0..H {
            for x in 0..W {
                // Coins lie flat-ish: squash the disc vertically.
                let (dx, dy) = (x as f32 + 0.5 - cx, (y as f32 + 0.5 - cy) * 1.6);
                let d = (dx * dx + dy * dy).sqrt();
                if d > r {
                    continue;
                }
                let rgb: [u8; 3] = if d > r - 0.9 {
                    [122, 74, 18] // rim
                } else if dx < -0.3 && dy < -0.3 && d < r * 0.55 {
                    [255, 240, 160] // highlight
                } else {
                    [236, 178, 52] // face
                };
                let i = ((y * W + x) * 4) as usize;
                data[i..i + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
            }
        }
    }
    Image::new(
        Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

/// The heaps' material, and which look (place and stage) it was made for.
struct HeapMaterial {
    handle: Handle<GradedMaterial>,
    look: (zrpg_core::Place, Option<zrpg_core::Theme>),
}

/// Keep one heap per pile of gold on the floor.
fn sync(
    mut commands: Commands,
    run: Res<Run>,
    assets: Res<LootAssets>,
    ground: Res<GroundHeights>,
    mut materials: ResMut<Assets<GradedMaterial>>,
    heaps: Query<(Entity, &Heap)>,
    mut material: Local<Option<HeapMaterial>>,
) {
    // One material per look, so the heaps are graded like their floor.
    let key = (run.floor.place, run.floor.stage.map(|s| s.theme));
    if material.as_ref().is_none_or(|m| m.look != key) {
        let handle = materials.add(Look::of(&run.floor).graded(StandardMaterial {
            base_color_texture: Some(assets.texture.clone()),
            alpha_mode: AlphaMode::Mask(0.5),
            emissive: GLINT,
            perceptual_roughness: 0.35,
            metallic: 0.6,
            cull_mode: None,
            ..default()
        }));
        *material = Some(HeapMaterial { handle, look: key });
    }
    let handle = material
        .as_ref()
        .map(|m| m.handle.clone())
        .unwrap_or_default();
    for (entity, heap) in &heaps {
        if !run.drops.iter().any(|d| d.pos == heap.0) {
            commands.entity(entity).despawn();
        }
    }
    for drop in &run.drops {
        if heaps.iter().any(|(_, h)| h.0 == drop.pos) {
            continue;
        }
        let at = cell_center(drop.pos);
        commands.spawn((
            Heap(drop.pos),
            Mesh3d(assets.mesh.clone()),
            MeshMaterial3d(handle.clone()),
            Transform::from_translation(at + Vec3::Y * ground.at(at)),
            NotShadowCaster,
        ));
    }
}

/// Heaps turn to face you, like everything else drawn flat.
fn turn_to_camera(
    cams: Query<&Transform, (With<PlayerCamera>, Without<Heap>)>,
    mut heaps: Query<&mut Transform, With<Heap>>,
) {
    let Ok(cam) = cams.single() else {
        return;
    };
    let (yaw, _, _) = cam.rotation.to_euler(EulerRot::YXZ);
    for mut t in &mut heaps {
        t.rotation = Quat::from_rotation_y(yaw);
    }
}
