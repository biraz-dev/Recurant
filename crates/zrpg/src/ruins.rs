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
//!
//! Dressing, all from the kit's art: some walls and columns are mossy;
//! ferns grow at the feet of walls and vines hang down them (alpha-masked
//! cards, no collision, clear of the cell centres you walk through); the
//! pools drift; and a sky dome fades from the haze at the horizon to blue.

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::light::{CascadeShadowConfigBuilder, NotShadowCaster};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use zrpg_core::rng::mix64;
use zrpg_core::ruins::{Ground, ground};
use zrpg_core::{Facing, GridPos};

use crate::atmosphere::{GradedMaterial, Look, untonemapped};
use crate::level::{cell_center, grid_dir};
use crate::textures;
use crate::view::PlayerCamera;

/// Kit metres to game units: one 2 m kit cell is one game cell.
pub const SCALE: f32 = 0.5;

const LEVEL: &[u8] = include_bytes!("../assets/ruins/level.bin");
const SANDSTONE: &[u8] = include_bytes!("../assets/ruins/sandstone.png");
const FLOOR: &[u8] = include_bytes!("../assets/ruins/floor.png");
const RUBBLE: &[u8] = include_bytes!("../assets/ruins/rubble.png");
const WATER: &[u8] = include_bytes!("../assets/ruins/water.png");
const MOSSY: &[u8] = include_bytes!("../assets/ruins/mossy.png");
const FERN: &[u8] = include_bytes!("../assets/ruins/fern.png");
const VINES: &[u8] = include_bytes!("../assets/ruins/vines.png");

/// Material slots in `level.bin`, as written by `tools/make_ruins.py`.
const SLOTS: usize = 5;
/// Which merged mesh each slot joins. M_Carved (2) shares the sandstone.
const SLOT_MESH: [usize; SLOTS] = [0, 1, 0, 2, 3];
const MESHES: usize = 5;
const WATER_MESH: usize = 3;
/// Masonry of a mossy instance joins this mesh instead of the sandstone.
const MOSSY_MESH: usize = 4;
/// Share of walls and columns that are damp and mossy.
const MOSSY_SHARE: f32 = 0.3;

/// Share of wall faces with a fern at their foot, and with vines down them.
const FERN_SHARE: f32 = 0.45;
const VINES_SHARE: f32 = 0.4;
/// Fern cluster size (it is wider than tall), and how far from the cell
/// centre it grows, towards the wall.
const FERN_SIZE: Vec2 = Vec2::new(0.5, 0.36);
const FERN_OUT: f32 = 0.34;
/// Vine card size; it hangs from the top of the 4 m wall.
const VINES_SIZE: Vec2 = Vec2::new(0.62, 0.95);
/// The walls are 0.2 m thick, centred on the cell edge; cards sit just off
/// their face.
const WALL_FACE: f32 = 0.5 - 0.2 * SCALE / 2.0 - 0.012;
const WALL_TOP: f32 = 4.0 * SCALE;

/// The water's slow drift, in texture repeats per second.
const WATER_DRIFT: Vec2 = Vec2::new(0.018, 0.011);

/// The sky: the haze at the horizon, rising to a soft blue overhead.
const SKY_ZENITH: Color = Color::srgb(0.55, 0.69, 0.86);
/// Inside the far clipping distance, beyond everything in the ruin.
const SKY_RADIUS: f32 = 30.0;

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

/// The kit's floor and bridge slabs stand a few millimetres apart; looking
/// straight down a seam, you would see the sky through it. This fill, just
/// below the deepest slab, shows instead, dark in the slabs' shadow.
const FILL_DEPTH: f32 = -0.85 * SCALE;

#[derive(Resource)]
pub struct RuinsAssets {
    meshes: Vec<(Handle<Mesh>, Handle<GradedMaterial>)>,
    fill: (Handle<Mesh>, Handle<GradedMaterial>),
    foliage: Vec<(Handle<Mesh>, Handle<GradedMaterial>)>,
    sky: (Handle<Mesh>, Handle<StandardMaterial>),
    water: Handle<GradedMaterial>,
}

/// Marks the sky dome, which travels with the camera.
#[derive(Component)]
pub struct SkyDome;

pub fn load(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<GradedMaterial>>,
    mut plain: ResMut<Assets<StandardMaterial>>,
) {
    let look = Look::RUINS;
    let mut material = |png: &[u8], roughness: f32| {
        let mut image = textures::with_mipmaps(textures::embedded(png));
        // Tiles repeat: one texture per 2 m, so 4 m walls show it twice.
        // Crisp pixels up close; mipmaps calm the shimmer far away.
        image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            address_mode_u: ImageAddressMode::Repeat,
            address_mode_v: ImageAddressMode::Repeat,
            mag_filter: ImageFilterMode::Nearest,
            min_filter: ImageFilterMode::Nearest,
            mipmap_filter: ImageFilterMode::Linear,
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
        material(MOSSY, 0.95),
    ];
    let water = mats[WATER_MESH].clone();
    let mut cutout = |png: &[u8]| {
        materials.add(look.graded(StandardMaterial {
            base_color_texture: Some(images.add(textures::embedded(png))),
            alpha_mode: AlphaMode::Mask(0.5),
            cull_mode: None,
            perceptual_roughness: 0.85,
            reflectance: 0.2,
            ..default()
        }))
    };
    let (fern_mat, vines_mat) = (cutout(FERN), cutout(VINES));
    let Parsed {
        meshes: merged,
        wall_faces,
    } = parse(LEVEL);
    let (ferns, vines) = foliage(&wall_faces);
    let sky = plain.add(StandardMaterial {
        unlit: true,
        fog_enabled: false,
        cull_mode: None,
        ..default()
    });
    let (w, d) = (18.0, 20.0);
    let fill = Plane3d::new(Vec3::Y, Vec2::new(w / 2.0, d / 2.0))
        .mesh()
        .build()
        .translated_by(Vec3::new(w / 2.0, FILL_DEPTH, d / 2.0));
    commands.insert_resource(RuinsAssets {
        fill: (meshes.add(fill), mats[1].clone()),
        foliage: vec![
            (meshes.add(ferns), fern_mat),
            (meshes.add(vines), vines_mat),
        ],
        sky: (meshes.add(sky_dome(&look)), sky),
        water,
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
    let (mesh, material) = &assets.fill;
    parent.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(material.clone()),
        NotShadowCaster,
    ));
    // Foliage casts its leafy shadows (alpha-masked, not solid cards).
    for (mesh, material) in &assets.foliage {
        parent.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone())));
    }
    let (mesh, material) = &assets.sky;
    parent.spawn((
        SkyDome,
        Mesh3d(mesh.clone()),
        MeshMaterial3d(material.clone()),
        NotShadowCaster,
        Transform::default(),
    ));
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

/// The pools drift slowly.
pub fn animate_water(
    time: Res<Time>,
    assets: Option<Res<RuinsAssets>>,
    mut materials: ResMut<Assets<GradedMaterial>>,
) {
    let Some(assets) = assets else {
        return;
    };
    let offset = (WATER_DRIFT * time.elapsed_secs()).fract();
    if let Some(mut m) = materials.get_mut(&assets.water) {
        m.base.uv_transform = bevy::math::Affine2::from_translation(offset);
    }
}

/// The sky dome stays centred on the camera, so its horizon is always the
/// true horizon.
pub fn follow_sky(
    cams: Query<&Transform, (With<PlayerCamera>, Without<SkyDome>)>,
    mut domes: Query<&mut Transform, With<SkyDome>>,
) {
    let Ok(cam) = cams.single() else {
        return;
    };
    for mut t in &mut domes {
        t.translation = cam.translation;
    }
}

/// A sphere coloured by height: the haze at and below the horizon, blending
/// to blue overhead. Colours are set before tonemapping, like the haze.
fn sky_dome(look: &Look) -> Mesh {
    let horizon = untonemapped(look.sky).to_linear();
    let zenith = untonemapped(SKY_ZENITH).to_linear();
    let mut mesh = Sphere::new(SKY_RADIUS).mesh().uv(32, 16);
    let colors: Vec<[f32; 4]> = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(bevy::mesh::VertexAttributeValues::Float32x3(ps)) => ps
            .iter()
            .map(|p| {
                let up = (p[1] / SKY_RADIUS).max(0.0);
                // Most of the change happens low, as in a real sky.
                let t = up.powf(0.6);
                horizon.mix(&zenith, t).to_f32_array()
            })
            .collect(),
        _ => unreachable!("sphere meshes have positions"),
    };
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh
}

/// Ferns at the feet of walls and vines down them, picked by a hash of the
/// wall face so the ruin always looks the same.
fn foliage(faces: &[(GridPos, Facing)]) -> (Mesh, Mesh) {
    let mut ferns = Cards::default();
    let mut vines = Cards::default();
    for &(cell, dir) in faces {
        let roll = |salt: u64| {
            let h = mix64((cell.x as u64) << 40 ^ (cell.y as u64) << 20 ^ (dir as u64) << 8 ^ salt);
            (h % 10_000) as f32 / 10_000.0
        };
        let toward = grid_dir(dir);
        let along = grid_dir(dir.right());
        let floor = cell_center(cell) + Vec3::Y * surface(cell);
        if ground(cell) == Ground::Dry && roll(1) < FERN_SHARE {
            // Two crossed cards, angled to the wall, nudged along it.
            let at = floor + toward * FERN_OUT + along * (roll(2) - 0.5) * 0.5;
            let scale = 0.8 + 0.4 * roll(3);
            for k in [-1.0, 1.0] {
                let across = (along + toward * k).normalize();
                ferns.card(at, across, FERN_SIZE * scale, false);
            }
        }
        if roll(4) < VINES_SHARE {
            let at = cell_center(cell)
                + toward * WALL_FACE
                + along * (roll(5) - 0.5) * 0.45
                + Vec3::Y * WALL_TOP;
            let length = 0.75 + 0.5 * roll(6);
            vines.card(at, along, VINES_SIZE * Vec2::new(1.0, length), true);
        }
    }
    (ferns.into_mesh(), vines.into_mesh())
}

/// Upright textured cards, built into one mesh.
#[derive(Default)]
struct Cards {
    data: MeshData,
}

impl Cards {
    /// A card centred on `at` horizontally, spanning along `across`. It
    /// stands up from `at`, or hangs down from it.
    fn card(&mut self, at: Vec3, across: Vec3, size: Vec2, hanging: bool) {
        let half = across * size.x / 2.0;
        let (bottom, top) = if hanging {
            (at - Vec3::Y * size.y, at)
        } else {
            (at, at + Vec3::Y * size.y)
        };
        let normal = across.cross(Vec3::Y).normalize();
        let d = &mut self.data;
        let base = d.positions.len() as u32;
        for (p, uv) in [
            (bottom - half, [0.0, 1.0]),
            (bottom + half, [1.0, 1.0]),
            (top + half, [1.0, 0.0]),
            (top - half, [0.0, 0.0]),
        ] {
            d.positions.push(p.to_array());
            d.normals.push(normal.to_array());
            d.uvs.push(uv);
        }
        d.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    fn into_mesh(self) -> Mesh {
        self.data.into_mesh()
    }
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

impl MeshData {
    fn into_mesh(self) -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
        .with_inserted_indices(Indices::U32(self.indices))
    }
}

/// What `level.bin` holds, ready to use.
struct Parsed {
    /// One merged mesh per material.
    meshes: Vec<Mesh>,
    /// Walkable cells beside a wall, and which way the wall is.
    wall_faces: Vec<(GridPos, Facing)>,
}

/// Place every module instance and merge the lot by material.
fn parse(bytes: &[u8]) -> Parsed {
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
    for i in 0..r.u32() {
        let module = r.u32() as usize;
        let mossy = (mix64(i as u64 ^ 0x6d05) % 1000) as f32 / 1000.0 < MOSSY_SHARE;
        let [m] = r.f32s::<12>(1).try_into().unwrap();
        let linear = Mat3::from_cols_slice(&m[..9]);
        let translation = Vec3::from_slice(&m[9..]);
        let normal_matrix = linear.inverse().transpose();
        // A mirroring transform would turn faces inside out.
        let flip = linear.determinant() < 0.0;
        for prim in &modules[module] {
            let slot_mesh = SLOT_MESH[prim.slot];
            let mesh = &mut out[if mossy && slot_mesh == 0 {
                MOSSY_MESH
            } else {
                slot_mesh
            }];
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
    let wall_faces = (0..r.u32())
        .map(|_| {
            let x = r.u32() as i32;
            let y = r.u32() as i32;
            (GridPos::new(x, y), Facing::ALL[r.u32() as usize])
        })
        .collect();
    assert_eq!(r.at, bytes.len(), "ruins level.bin has trailing data");
    Parsed {
        meshes: out.into_iter().map(MeshData::into_mesh).collect(),
        wall_faces,
    }
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
        let meshes = parse(LEVEL).meshes;
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

    /// Dressing only goes where the kit put walls, beside cells you walk.
    #[test]
    fn foliage_grows_against_real_walls() {
        let faces = parse(LEVEL).wall_faces;
        assert!(faces.len() > 30, "{} wall faces", faces.len());
        let floor = zrpg_core::ruins::floor();
        for &(cell, dir) in &faces {
            assert!(floor.map.is_walkable(cell), "{cell:?}");
            assert!(!floor.map.is_walkable(cell + dir), "{cell:?} {dir:?}");
        }
        // The start corridor is lined with walls on both sides.
        let start = GridPos::new(2, 16);
        assert!(faces.contains(&(start, Facing::West)));
    }

    #[test]
    fn surfaces_step_down_into_the_pool() {
        assert_eq!(surface(GridPos::new(2, 17)), 0.0);
        assert_eq!(surface(GridPos::new(10, 11)), -0.15);
        assert_eq!(surface(GridPos::new(8, 12)), -0.3);
    }
}
