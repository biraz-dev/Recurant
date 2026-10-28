//! The sandstone ruins: the run's first floor, built from the ruins asset kit
//! (`art/kits/sandstone-ruins-v1`). `tools/make_ruins.py` converts the kit's
//! eleven module meshes and its placement layout into `assets/ruins/level.bin`;
//! here every placed module is merged into one mesh per material, so the
//! whole ruin (about 39k triangles) draws in four calls.
//!
//! The kit works in metres on a 2 m grid; the game's cells are 1.0, so
//! everything is scaled by [`SCALE`]. Walls stand 2.0 (4 m) here, twice a
//! dungeon wall: the ruins are meant to feel open and tall.
//!
//! Materials follow the kit's contract: basecolor only (no normal or
//! roughness maps are supplied), high roughness, shared per material, with
//! repeat addressing for its tile candidates. The carved slot uses the plain
//! sandstone, as the kit asks, until its motif sheet gets authored UVs.

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::light::{CascadeShadowConfigBuilder, NotShadowCaster};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use zrpg_core::GridPos;
use zrpg_core::ruins::{Ground, ground};

use crate::atmosphere::{GradedMaterial, Look};
use crate::textures;

/// Kit metres to game units: one 2 m kit cell is one game cell.
pub const SCALE: f32 = 0.5;

const LEVEL: &[u8] = include_bytes!("../assets/ruins/level.bin");
const SANDSTONE: &[u8] = include_bytes!("../assets/ruins/sandstone.png");
const FLOOR: &[u8] = include_bytes!("../assets/ruins/floor.png");
const RUBBLE: &[u8] = include_bytes!("../assets/ruins/rubble.png");
const WATER: &[u8] = include_bytes!("../assets/ruins/water.png");

/// Material slots in `level.bin`, as written by `tools/make_ruins.py`.
const SLOTS: usize = 5;
/// Which merged mesh each slot joins. M_Carved (2) shares the sandstone.
const SLOT_MESH: [usize; SLOTS] = [0, 1, 0, 2, 3];
const MESHES: usize = 4;
const WATER_MESH: usize = 3;

/// Afternoon sun, from high over the east: the corridor's roof and the
/// court walls throw real shadows, as in the kit's reference.
const SUN_COLOR: Color = Color::srgb(1.0, 0.9, 0.74);
const SUN_LUX: f32 = 1100.0;
const SUN_TRAVEL: Vec3 = Vec3::new(-0.55, -0.78, -0.3);

/// Ground height at the centre of a ruins cell, in game units: the pool's
/// bed is 0.6 m down, and its stairs climb halfway by their centre.
pub fn surface(p: GridPos) -> f32 {
    SCALE
        * match ground(p) {
            Ground::Water => -0.6,
            Ground::Stairs => -0.3,
            Ground::Solid | Ground::Dry | Ground::Bridge => 0.0,
        }
}

#[derive(Resource)]
pub struct RuinsAssets {
    meshes: Vec<(Handle<Mesh>, Handle<GradedMaterial>)>,
}

pub fn load(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<GradedMaterial>>,
) {
    let look = Look::RUINS;
    let mut material = |png: &[u8], roughness: f32| {
        let mut image = textures::embedded(png);
        // Tiles repeat: one texture per 2 m, so 4 m walls show it twice.
        image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            address_mode_u: ImageAddressMode::Repeat,
            address_mode_v: ImageAddressMode::Repeat,
            mag_filter: ImageFilterMode::Nearest,
            min_filter: ImageFilterMode::Nearest,
            ..default()
        });
        materials.add(look.graded(StandardMaterial {
            base_color_texture: Some(images.add(image)),
            perceptual_roughness: roughness,
            metallic: 0.0,
            reflectance: 0.25,
            ..default()
        }))
    };
    let mats = [
        material(SANDSTONE, 0.92),
        material(FLOOR, 0.9),
        material(RUBBLE, 0.95),
        // Opaque, as the kit suggests; a little sheen catches the sun.
        material(WATER, 0.35),
    ];
    let merged = merge(LEVEL);
    commands.insert_resource(RuinsAssets {
        meshes: merged
            .into_iter()
            .zip(mats)
            .map(|(mesh, mat)| (meshes.add(mesh), mat))
            .collect(),
    });
}

/// The ruin's geometry and its sun, as children of the level.
pub fn spawn(parent: &mut ChildSpawnerCommands, assets: &RuinsAssets) {
    for (i, (mesh, material)) in assets.meshes.iter().enumerate() {
        let mut e = parent.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone())));
        if i == WATER_MESH {
            e.insert(NotShadowCaster);
        }
    }
    parent.spawn((
        DirectionalLight {
            color: SUN_COLOR,
            illuminance: SUN_LUX,
            shadow_maps_enabled: true,
            ..default()
        },
        // One cascade covering what fog leaves visible keeps shadows cheap.
        CascadeShadowConfigBuilder {
            num_cascades: 1,
            maximum_distance: 22.0,
            ..default()
        }
        .build(),
        Transform::IDENTITY.looking_to(SUN_TRAVEL, Vec3::Y),
    ));
}

/// Little-endian reader for `level.bin`.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn take(&mut self, n: usize) -> &[u8] {
        let s = &self.bytes[self.at..self.at + n];
        self.at += n;
        s
    }

    fn u32(&mut self) -> u32 {
        u32::from_le_bytes(self.take(4).try_into().unwrap())
    }

    fn f32s<const N: usize>(&mut self, count: usize) -> Vec<[f32; N]> {
        (0..count)
            .map(|_| core::array::from_fn(|_| f32::from_le_bytes(self.take(4).try_into().unwrap())))
            .collect()
    }

    fn u16s(&mut self, count: usize) -> Vec<u16> {
        let v = (0..count)
            .map(|_| u16::from_le_bytes(self.take(2).try_into().unwrap()))
            .collect();
        // Index lists are padded to four bytes.
        if count % 2 == 1 {
            self.take(2);
        }
        v
    }
}

struct Primitive {
    slot: usize,
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u16>,
}

#[derive(Default)]
struct MeshData {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

/// Place every module instance and merge the lot by material.
fn merge(bytes: &[u8]) -> Vec<Mesh> {
    let mut r = Reader { bytes, at: 0 };
    assert_eq!(r.take(4), b"ZRL1", "ruins level.bin header");
    let modules: Vec<Vec<Primitive>> = (0..r.u32())
        .map(|_| {
            (0..r.u32())
                .map(|_| {
                    let slot = r.u32() as usize;
                    let n = r.u32() as usize;
                    let positions = r.f32s::<3>(n);
                    let normals = r.f32s::<3>(n);
                    let uvs = r.f32s::<2>(n);
                    let count = r.u32() as usize;
                    Primitive {
                        slot,
                        positions,
                        normals,
                        uvs,
                        indices: r.u16s(count),
                    }
                })
                .collect()
        })
        .collect();

    let mut out: Vec<MeshData> = (0..MESHES).map(|_| MeshData::default()).collect();
    for _ in 0..r.u32() {
        let module = r.u32() as usize;
        let [m] = r.f32s::<12>(1).try_into().unwrap();
        let linear = Mat3::from_cols_slice(&m[..9]);
        let translation = Vec3::from_slice(&m[9..]);
        let normal_matrix = linear.inverse().transpose();
        // A mirroring transform would turn faces inside out.
        let flip = linear.determinant() < 0.0;
        for prim in &modules[module] {
            let mesh = &mut out[SLOT_MESH[prim.slot]];
            let base = mesh.positions.len() as u32;
            for ((p, n), uv) in prim.positions.iter().zip(&prim.normals).zip(&prim.uvs) {
                let world = (linear * Vec3::from_array(*p) + translation) * SCALE;
                mesh.positions.push(world.to_array());
                mesh.normals.push(
                    (normal_matrix * Vec3::from_array(*n))
                        .normalize()
                        .to_array(),
                );
                mesh.uvs.push(*uv);
            }
            for tri in prim.indices.chunks_exact(3) {
                let [a, b, c] = [tri[0], tri[1], tri[2]].map(|i| base + i as u32);
                mesh.indices
                    .extend_from_slice(&if flip { [a, c, b] } else { [a, b, c] });
            }
        }
    }
    assert_eq!(r.at, bytes.len(), "ruins level.bin has trailing data");
    out.into_iter()
        .map(|m| {
            Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::RENDER_WORLD,
            )
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, m.positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, m.normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, m.uvs)
            .with_inserted_indices(Indices::U32(m.indices))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use bevy::mesh::VertexAttributeValues;

    use super::*;

    fn positions(mesh: &Mesh) -> Vec<Vec3> {
        match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
            Some(VertexAttributeValues::Float32x3(v)) => {
                v.iter().map(|p| Vec3::from_array(*p)).collect()
            }
            _ => panic!("positions"),
        }
    }

    /// The kit's numbers survive conversion: about 39k triangles, the grid
    /// 18 x 20 cells, and heights in metres halved.
    #[test]
    fn ruins_land_on_the_game_grid() {
        let meshes = merge(LEVEL);
        let triangles: usize = meshes.iter().map(|m| m.indices().unwrap().len() / 3).sum();
        // 39,336 in the kit's full layout, less the floor slab the stairs replace.
        assert_eq!(triangles, 39_160);
        let all: Vec<Vec3> = meshes.iter().flat_map(positions).collect();
        let (min, max) = all.iter().fold((Vec3::MAX, Vec3::MIN), |(lo, hi), p| {
            (lo.min(*p), hi.max(*p))
        });
        assert!(min.x >= -0.01 && max.x <= 18.01, "{min} {max}");
        assert!(min.z >= -0.01 && max.z <= 20.01, "{min} {max}");
        // The corridor's ceiling slabs sit at 4 m and are 0.2 m thick.
        assert!((max.y - 4.2 * SCALE).abs() < 0.01, "{max}");
        // The pool bed is the lowest walking surface; slabs are 0.2 m thick.
        assert!((min.y - (-0.8 * SCALE)).abs() < 0.01, "{min}");
    }

    #[test]
    fn surfaces_step_down_into_the_pool() {
        assert_eq!(surface(GridPos::new(2, 17)), 0.0);
        assert_eq!(surface(GridPos::new(10, 11)), -0.15);
        assert_eq!(surface(GridPos::new(8, 12)), -0.3);
    }
}
