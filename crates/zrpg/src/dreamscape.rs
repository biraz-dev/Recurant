//! Dreamscape floors, drawn from the Dreamscape modular kits
//! (`art/kits/dreamscape-modular-v1`). For now the Mindglass Sanctum, floor
//! 2: `tools/make_dreamscape.py` lays it out from the kit's modules and
//! writes `assets/dream/mindglass-sanctum/` (the placed modules, their four
//! role textures, and its crystal lights); `zrpg_core::dream` has its map.
//!
//! Materials follow the kit's contract: the four roles bind by name to its
//! diffuse candidates (no normal or roughness maps are supplied, so none are
//! invented); wall and ground repeat every 2 m, structure and detail panels
//! clamp. Crisp pixels up close, mipmaps far away. The detail role (crystal,
//! gold tracery) glows faintly, and the crystals' lights breathe slowly.

use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;

use crate::atmosphere::{GradedMaterial, Look};
use crate::kit_level::{self, MeshData, SCALE};
use crate::textures;

const LEVEL: &[u8] = include_bytes!("../assets/dream/mindglass-sanctum/level.bin");
/// The four roles, in the order `tools/make_dreamscape.py` numbers them.
const ROLES: [&[u8]; 4] = [
    include_bytes!("../assets/dream/mindglass-sanctum/wall.png"),
    include_bytes!("../assets/dream/mindglass-sanctum/ground.png"),
    include_bytes!("../assets/dream/mindglass-sanctum/structure.png"),
    include_bytes!("../assets/dream/mindglass-sanctum/detail.png"),
];
/// Which roles repeat (wall, ground); the others clamp, as panels.
const REPEATS: [bool; 4] = [true, true, false, false];
const ROUGHNESS: [f32; 4] = [0.85, 0.7, 0.8, 0.55];
/// How brightly each role glows by itself (the kit supplies no emission
/// map, so this is the colour texture, faintly).
const GLOW: [f32; 4] = [0.0, 0.0, 0.0, 0.12];
/// The crystals' lights: how far they reach, and how they breathe.
const GLOW_RANGE: f32 = 6.5;
const BREATH_HZ: f32 = 0.23;
const BREATH_DEPTH: f32 = 0.18;

#[derive(Resource)]
pub struct DreamAssets {
    meshes: Vec<(Handle<Mesh>, Handle<GradedMaterial>)>,
    /// Where the crystal lights are (game units), their colour and lumens.
    glows: Vec<(Vec3, Color, f32)>,
}

/// A breathing crystal light.
#[derive(Component)]
pub struct Glow {
    lumens: f32,
    phase: f32,
}

pub fn load(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<GradedMaterial>>,
) {
    let look = Look::SANCTUM;
    let mats: Vec<Handle<GradedMaterial>> = (0..ROLES.len())
        .map(|role| {
            let mut image = textures::with_mipmaps(textures::embedded(ROLES[role]));
            let address = if REPEATS[role] {
                ImageAddressMode::Repeat
            } else {
                ImageAddressMode::ClampToEdge
            };
            image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: address,
                address_mode_v: address,
                mag_filter: ImageFilterMode::Nearest,
                min_filter: ImageFilterMode::Nearest,
                mipmap_filter: ImageFilterMode::Linear,
                ..default()
            });
            let texture = images.add(image);
            let glow = GLOW[role];
            materials.add(look.graded(StandardMaterial {
                base_color_texture: Some(texture.clone()),
                emissive: LinearRgba::rgb(glow, glow, glow),
                emissive_texture: (glow > 0.0).then_some(texture),
                perceptual_roughness: ROUGHNESS[role],
                metallic: 0.0,
                reflectance: 0.3,
                ..default()
            }))
        })
        .collect();
    let (merged, mut r) = kit_level::parse(LEVEL, ROLES.len(), |slot, _| slot);
    let glows = (0..r.u32())
        .map(|_| {
            let [v] = r.f32s::<7>(1).try_into().unwrap();
            (
                Vec3::new(v[0], v[1], v[2]) * SCALE,
                Color::linear_rgb(v[3], v[4], v[5]),
                v[6],
            )
        })
        .collect();
    assert!(r.is_done(), "dreamscape level.bin has trailing data");
    commands.insert_resource(DreamAssets {
        meshes: merged
            .into_iter()
            .map(MeshData::into_mesh)
            .zip(mats)
            .map(|(mesh, mat)| (meshes.add(mesh), mat))
            .collect(),
        glows,
    });
}

/// The sanctum's geometry and its crystal lights, as children of the level.
pub fn spawn(parent: &mut ChildSpawnerCommands, assets: &DreamAssets) {
    for (mesh, material) in &assets.meshes {
        parent.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone())));
    }
    for (i, &(at, color, lumens)) in assets.glows.iter().enumerate() {
        parent.spawn((
            Glow {
                lumens,
                phase: i as f32 * 1.7,
            },
            PointLight {
                color,
                intensity: lumens,
                range: GLOW_RANGE,
                radius: 0.1,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_translation(at),
        ));
    }
}

/// The crystals' light swells and fades, slowly and out of step.
pub fn breathe(time: Res<Time>, mut lights: Query<(&Glow, &mut PointLight)>) {
    let t = time.elapsed_secs();
    for (glow, mut light) in &mut lights {
        let breath = (t * BREATH_HZ * std::f32::consts::TAU + glow.phase).sin();
        light.intensity = glow.lumens * (1.0 + BREATH_DEPTH * breath);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The sanctum lands on the game grid: the 15 x 16 cell layout, its
    /// coffered ceiling up at 4.26 m (halved), its floor slabs' tops at 0,
    /// and its lights in the cells where the crystals stand.
    #[test]
    fn the_sanctum_lands_on_the_game_grid() {
        let (meshes, mut r) = kit_level::parse(LEVEL, ROLES.len(), |slot, _| slot);
        let all: Vec<[f32; 3]> = meshes.iter().flat_map(|m| m.positions.clone()).collect();
        let triangles: usize = meshes.iter().map(|m| m.indices.len() / 3).sum();
        assert!((13_000..15_000).contains(&triangles), "{triangles}");
        let (lo, hi) = all.iter().fold((Vec3::MAX, Vec3::MIN), |(lo, hi), p| {
            let p = Vec3::from_array(*p);
            (lo.min(p), hi.max(p))
        });
        assert!(lo.x >= 0.9 && hi.x <= 14.1, "{lo} {hi}");
        assert!(lo.z >= 0.9 && hi.z <= 15.1, "{lo} {hi}");
        assert!((hi.y - 4.26 * SCALE).abs() < 0.01, "{hi}");
        // Floor slabs are 0.2 m thick, top at 0.
        assert!((lo.y - (-0.2 * SCALE)).abs() < 0.01, "{lo}");
        let floor = zrpg_core::dream::floor();
        let lights = r.u32();
        assert_eq!(lights, 7);
        for _ in 0..lights {
            let [v] = r.f32s::<7>(1).try_into().unwrap();
            let cell = zrpg_core::GridPos::new((v[0] / 2.0) as i32, (v[2] / 2.0) as i32);
            assert!(
                !floor.map.is_walkable(cell),
                "a light in the lane at {cell:?}"
            );
        }
        assert!(r.is_done());
    }
}
