//! Turns a `zrpg_core::Map` into geometry: one mesh per material, so a whole
//! floor costs about a dozen draw calls.
//!
//! World layout: grid cell `(x, y)` spans `x..x+1` on X and `y..y+1` on Z,
//! floor at height 0, ceiling at `WALL_HEIGHT`.
//!
//! Dressing (which walls are mossy, cracked or carved, where vines hang, ferns
//! grow and torches burn) is picked from a hash of the floor seed and the
//! cell, so a floor always looks the same.
//!
//! Surfaces are lit (see `lighting.rs`): every material has a normal map and a
//! roughness map, and the meshes carry tangents so the normal maps line up.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use zrpg_core::rng::mix64;
use zrpg_core::{Facing, GridPos, Map, Tile};

use crate::Run;
use crate::lighting::{self, TorchAssets};
use crate::textures::{self, Maps, env};

pub const WALL_HEIGHT: f32 = 1.0;
/// Stairs: a pit of steps going down under the far wall.
const STAIR_STEPS: usize = 4;
const STAIR_DROP: f32 = 0.1;

/// Wall variants, in the order of `env::WALLS`: plain, mossy, cracked, carved.
const WALL_KINDS: usize = 4;
/// Chance of each wall variant except plain (cumulative order: carved,
/// cracked, mossy).
const CARVED: f32 = 0.04;
const CRACKED: f32 = 0.12;
const MOSSY_WALL: f32 = 0.22;
/// Floor variants, in the order of `env::FLOORS`: plain, mossy.
const FLOOR_KINDS: usize = 2;
const MOSSY_FLOOR: f32 = 0.3;
const VINES: f32 = 0.28;
/// Vines hang from the ceiling down to this height.
const VINES_BOTTOM: f32 = 0.35;
/// Decals sit this far in front of the wall so they never flicker.
const DECAL_OFFSET: f32 = 0.004;
const FERN: f32 = 0.2;
const FERN_SIZE: f32 = 0.42;
/// Wall torches: candidates per wall face, how many per floor at most, and
/// how far apart (in cells) they must be.
const TORCH: f32 = 0.1;
const MAX_TORCHES: usize = 4;
const TORCH_SPACING: i32 = 4;
/// The sconce (bracket and torch) on the wall: size and height of its foot.
const SCONCE_SIZE: Vec2 = Vec2::new(0.2, 0.3);
const SCONCE_BOTTOM: f32 = 0.42;

/// A wall torch: where its flame burns and which way its wall faces.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Torch {
    pub flame: Vec3,
    pub facing: Vec3,
}

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_materials)
            .add_systems(Update, rebuild_level);
    }
}

#[derive(Resource)]
struct LevelMaterials {
    walls: [Handle<StandardMaterial>; WALL_KINDS],
    floors: [Handle<StandardMaterial>; FLOOR_KINDS],
    ceiling: Handle<StandardMaterial>,
    stairs: Handle<StandardMaterial>,
    arch: Handle<StandardMaterial>,
    vines: Handle<StandardMaterial>,
    fern: Handle<StandardMaterial>,
    sconce: Handle<StandardMaterial>,
}

/// Parent of all geometry for the current floor.
#[derive(Component)]
struct LevelRoot {
    run_seed: u64,
    depth: u32,
}

fn load_materials(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Cutouts (vines, ferns, sconces) are alpha-masked and visible from both
    // sides. The ORM map serves as both occlusion and roughness (glTF packing).
    let mut make = |maps: Maps, cutout: bool| {
        let orm = images.add(textures::embedded_linear(maps.orm));
        materials.add(StandardMaterial {
            base_color_texture: Some(images.add(textures::embedded(maps.color))),
            normal_map_texture: Some(images.add(textures::embedded_linear(maps.normal))),
            metallic_roughness_texture: Some(orm.clone()),
            occlusion_texture: Some(orm),
            perceptual_roughness: 1.0,
            metallic: 0.0,
            reflectance: 0.3,
            alpha_mode: if cutout {
                AlphaMode::Mask(0.5)
            } else {
                AlphaMode::Opaque
            },
            cull_mode: if cutout {
                None
            } else {
                Some(bevy::render::render_resource::Face::Back)
            },
            ..default()
        })
    };
    commands.insert_resource(LevelMaterials {
        walls: env::WALLS.map(|maps| make(maps, false)),
        floors: env::FLOORS.map(|maps| make(maps, false)),
        ceiling: make(env::CEILING, false),
        stairs: make(env::STAIRS, false),
        arch: make(env::ARCH, false),
        vines: make(env::VINES, true),
        fern: make(env::FERN, true),
        sconce: make(env::SCONCE, true),
    });
}

fn rebuild_level(
    mut commands: Commands,
    run: Res<Run>,
    roots: Query<(Entity, &LevelRoot)>,
    mats: Res<LevelMaterials>,
    torch_assets: Res<TorchAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let current = roots
        .iter()
        .any(|(_, r)| r.run_seed == run.run_seed && r.depth == run.depth);
    if current {
        return;
    }
    for (entity, _) in &roots {
        commands.entity(entity).despawn();
    }

    let seed = mix64(run.run_seed ^ mix64(run.depth as u64));
    let geo = LevelGeometry::build(&run.floor.map, run.floor.stairs_dir, seed);
    commands
        .spawn((
            LevelRoot {
                run_seed: run.run_seed,
                depth: run.depth,
            },
            Transform::default(),
            Visibility::default(),
        ))
        .with_children(|parent| {
            let parts = geo
                .walls
                .into_iter()
                .zip(&mats.walls)
                .chain(geo.floors.into_iter().zip(&mats.floors))
                .chain([
                    (geo.ceilings, &mats.ceiling),
                    (geo.stairs, &mats.stairs),
                    (geo.arch, &mats.arch),
                    (geo.vines, &mats.vines),
                    (geo.ferns, &mats.fern),
                    (geo.sconces, &mats.sconce),
                ]);
            for (builder, material) in parts {
                if !builder.is_empty() {
                    parent.spawn((
                        Mesh3d(meshes.add(builder.into_mesh())),
                        MeshMaterial3d(material.clone()),
                    ));
                }
            }
            for (i, torch) in geo.torches.iter().enumerate() {
                lighting::spawn_wall_torch(parent, &torch_assets, torch, i as f32 * 2.3);
            }
        });
}

const FLOOR_BRIGHTNESS: f32 = 1.0;
const CEILING_BRIGHTNESS: f32 = 0.85;

/// Baked ambient occlusion: wall tops sit in the ceiling's shadow, and the
/// floor and ceiling darken where they meet walls. Free at runtime, and the
/// main source of depth in an unlit renderer.
const WALL_TOP_SHADE: f32 = 0.72;
const WALL_FOOT_SHADE: f32 = 0.9;
const CORNER_SHADE: f32 = 0.16;
const UV: [[f32; 2]; 4] = [[0., 1.], [1., 1.], [1., 0.], [0., 0.]];

/// How lit a floor or ceiling corner at grid point `(x, z)` is: darker for
/// each of the four cells meeting there that is solid.
fn corner_shade(map: &Map, x: i32, z: i32) -> f32 {
    let solid = [(x - 1, z - 1), (x, z - 1), (x - 1, z), (x, z)]
        .into_iter()
        .filter(|&(cx, cz)| !map.is_walkable(GridPos::new(cx, cz)))
        .count();
    1.0 - CORNER_SHADE * solid as f32
}

/// Stable pseudo-random number in `0..1` for a cell and a purpose.
fn roll(seed: u64, p: GridPos, salt: u64) -> f32 {
    let h = mix64(seed ^ ((p.x as u64) << 40) ^ ((p.y as u64) << 20) ^ salt);
    (h >> 40) as f32 / (1u64 << 24) as f32
}

#[derive(Default)]
struct QuadBuilder {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    tangents: Vec<[f32; 4]>,
    uvs: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl QuadBuilder {
    fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// Corners counter-clockwise as seen from the front:
    /// bottom-left, bottom-right, top-right, top-left.
    fn quad(&mut self, corners: [Vec3; 4], brightness: f32) {
        self.quad_uv(corners, UV, brightness);
    }

    /// Like [`quad`](Self::quad) with explicit texture coordinates, so thin
    /// faces (steps, risers) show a slice of the texture instead of a squash.
    fn quad_uv(&mut self, corners: [Vec3; 4], uvs: [[f32; 2]; 4], brightness: f32) {
        self.quad_shaded(corners, uvs, [brightness; 4]);
    }

    /// Full control: a brightness per corner, interpolated across the face.
    /// That is how the baked shadows (see [`corner_shade`]) are drawn.
    fn quad_shaded(&mut self, corners: [Vec3; 4], uvs: [[f32; 2]; 4], shade: [f32; 4]) {
        let normal = (corners[1] - corners[0])
            .cross(corners[2] - corners[0])
            .normalize();
        let tangent = quad_tangent(corners, uvs, normal);
        let base = self.positions.len() as u32;
        for ((corner, uv), b) in corners.iter().zip(uvs).zip(shade) {
            self.positions.push(corner.to_array());
            self.normals.push(normal.to_array());
            self.tangents.push(tangent.to_array());
            self.uvs.push(uv);
            self.colors.push([b, b, b, 1.0]);
        }
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    fn into_mesh(self) -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_TANGENT, self.tangents)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
        .with_inserted_indices(Indices::U32(self.indices))
    }
}

/// Tangent for a flat quad, from how its UVs run across it: `xyz` points
/// along increasing U; `w` makes Bevy's bitangent (`w * normal x tangent`)
/// point to the top of the texture (decreasing V), matching the normal maps'
/// OpenGL convention (+Y up the image).
fn quad_tangent(corners: [Vec3; 4], uvs: [[f32; 2]; 4], normal: Vec3) -> Vec4 {
    let (e1, e2) = (corners[1] - corners[0], corners[3] - corners[0]);
    let (du1, dv1) = (uvs[1][0] - uvs[0][0], uvs[1][1] - uvs[0][1]);
    let (du2, dv2) = (uvs[3][0] - uvs[0][0], uvs[3][1] - uvs[0][1]);
    let det = du1 * dv2 - du2 * dv1;
    if det.abs() < 1e-8 {
        return Vec4::new(1.0, 0.0, 0.0, 1.0);
    }
    let along_u = ((e1 * dv2 - e2 * dv1) / det).normalize();
    let along_v = (e2 * du1 - e1 * du2) / det;
    let w = if normal.cross(along_u).dot(-along_v) >= 0.0 {
        1.0
    } else {
        -1.0
    };
    along_u.extend(w)
}

#[derive(Default)]
struct LevelGeometry {
    walls: [QuadBuilder; WALL_KINDS],
    floors: [QuadBuilder; FLOOR_KINDS],
    ceilings: QuadBuilder,
    stairs: QuadBuilder,
    /// The wall the stairs lead under.
    arch: QuadBuilder,
    vines: QuadBuilder,
    ferns: QuadBuilder,
    sconces: QuadBuilder,
    torches: Vec<Torch>,
}

impl LevelGeometry {
    fn build(map: &Map, stairs_dir: Facing, seed: u64) -> Self {
        let mut geo = LevelGeometry::default();
        let h = WALL_HEIGHT;
        let mut torch_spots: Vec<(f32, GridPos, Facing)> = Vec::new();
        for p in map.positions().filter(|&p| map.is_walkable(p)) {
            let (x0, z0) = (p.x as f32, p.y as f32);
            let (x1, z1) = (x0 + 1.0, z0 + 1.0);

            let floor = [
                Vec3::new(x0, 0., z1),
                Vec3::new(x1, 0., z1),
                Vec3::new(x1, 0., z0),
                Vec3::new(x0, 0., z0),
            ];
            // Shade at the cell's corners: (x0,z0), (x1,z0), (x1,z1), (x0,z1).
            let ao = |x: i32, z: i32| corner_shade(map, x, z);
            let (nw, ne, se, sw) = (
                ao(p.x, p.y),
                ao(p.x + 1, p.y),
                ao(p.x + 1, p.y + 1),
                ao(p.x, p.y + 1),
            );
            let stairs = map.get(p) == Tile::StairsDown;
            if stairs {
                geo.add_stairs(p, stairs_dir);
            } else {
                let kind = usize::from(roll(seed, p, 1) < MOSSY_FLOOR);
                geo.floors[kind].quad_shaded(
                    floor,
                    UV,
                    [sw, se, ne, nw].map(|s| s * FLOOR_BRIGHTNESS),
                );
            }
            geo.ceilings.quad_shaded(
                [
                    Vec3::new(x0, h, z0),
                    Vec3::new(x1, h, z0),
                    Vec3::new(x1, h, z1),
                    Vec3::new(x0, h, z1),
                ],
                UV,
                [nw, ne, se, sw].map(|s| s * CEILING_BRIGHTNESS),
            );

            for dir in Facing::ALL {
                if map.is_walkable(p + dir) {
                    continue;
                }
                // Viewed from inside the cell looking towards `dir`.
                let forward = grid_dir(dir);
                let right = grid_dir(dir.right());
                let center = cell_center(p) + forward * 0.5;
                let (l, r) = (center - right * 0.5, center + right * 0.5);
                let light = 1.0;
                let wall = [l, r, r + Vec3::Y * h, l + Vec3::Y * h];
                if stairs && dir == stairs_dir {
                    let (foot, top) = (light * WALL_FOOT_SHADE, light * WALL_TOP_SHADE);
                    geo.arch.quad_shaded(wall, UV, [foot, foot, top, top]);
                    continue;
                }
                let salt = 10 + dir as u64;
                let kind = match roll(seed, p, salt) {
                    r if r < CARVED => 3,
                    r if r < CARVED + CRACKED => 2,
                    r if r < CARVED + CRACKED + MOSSY_WALL => 1,
                    _ => 0,
                };
                let foot = light * WALL_FOOT_SHADE;
                let top = light * WALL_TOP_SHADE;
                geo.walls[kind].quad_shaded(wall, UV, [foot, foot, top, top]);
                let t = roll(seed, p, salt + 30);
                if t < TORCH {
                    torch_spots.push((t, p, dir));
                }
                if roll(seed, p, salt + 10) < VINES {
                    let lift = -forward * DECAL_OFFSET;
                    let (l, r) = (l + lift, r + lift);
                    let bottom = Vec3::Y * VINES_BOTTOM;
                    let mid = light * (WALL_FOOT_SHADE + WALL_TOP_SHADE) * 0.5;
                    let top = light * WALL_TOP_SHADE;
                    geo.vines.quad_shaded(
                        [l + bottom, r + bottom, r + Vec3::Y * h, l + Vec3::Y * h],
                        UV,
                        [mid, mid, top, top],
                    );
                }
            }
            if !stairs && roll(seed, p, 30) < FERN {
                geo.add_fern(map, p, seed);
            }
        }
        // Torches: the luckiest candidates, kept well apart.
        torch_spots.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut placed: Vec<GridPos> = Vec::new();
        for (_, p, dir) in torch_spots {
            if placed.len() < MAX_TORCHES && placed.iter().all(|q| q.manhattan(p) >= TORCH_SPACING)
            {
                placed.push(p);
                geo.add_torch(p, dir);
            }
        }
        geo
    }

    /// A sconce on the wall of cell `p` facing `dir`, and its flame.
    fn add_torch(&mut self, p: GridPos, dir: Facing) {
        let (f, r) = (grid_dir(dir), grid_dir(dir.right()));
        let wall = cell_center(p) + f * (0.5 - DECAL_OFFSET * 2.0);
        let (half, up) = (r * SCONCE_SIZE.x * 0.5, Vec3::Y * SCONCE_SIZE.y);
        let foot = wall + Vec3::Y * SCONCE_BOTTOM;
        self.sconces.quad(
            [foot - half, foot + half, foot + half + up, foot - half + up],
            1.0,
        );
        self.torches.push(Torch {
            // Just above the sconce's cup, standing a little off the wall.
            flame: foot + up * 0.88 - f * 0.05,
            facing: -f,
        });
    }

    /// A fern tucked against one of the cell's walls: two quads crossed at
    /// right angles, the classic way to fake a bush.
    fn add_fern(&mut self, map: &Map, p: GridPos, seed: u64) {
        let walls: Vec<Facing> = Facing::ALL
            .into_iter()
            .filter(|&d| !map.is_walkable(p + d))
            .collect();
        if walls.is_empty() {
            return;
        }
        let dir = walls[(roll(seed, p, 31) * walls.len() as f32) as usize % walls.len()];
        let (f, r) = (grid_dir(dir), grid_dir(dir.right()));
        let base = cell_center(p) + f * 0.3 + r * ((roll(seed, p, 32) - 0.5) * 0.5);
        let half = FERN_SIZE * 0.5;
        for axis in [(f + r).normalize(), (f - r).normalize()] {
            let (a, b) = (base - axis * half, base + axis * half);
            let up = Vec3::Y * FERN_SIZE;
            self.ferns.quad([a, b, b + up, a + up], 0.9);
        }
    }

    /// Steps descending along `dir`, with pit walls on both sides and a
    /// black opening at the bottom.
    fn add_stairs(&mut self, p: GridPos, dir: Facing) {
        let (f, r) = (grid_dir(dir), grid_dir(dir.right()));
        let c = cell_center(p);
        // Cell-local point: `along` runs -0.5 (entry) to 0.5 (far edge).
        let at = |along: f32, side: f32, y: f32| c + f * along + r * side + Vec3::Y * y;
        let n = STAIR_STEPS as f32;
        for k in 0..STAIR_STEPS {
            let (a0, a1) = (k as f32 / n - 0.5, (k + 1) as f32 / n - 0.5);
            let (v0, v1) = (k as f32 / n, (k + 1) as f32 / n);
            let top = -(k as f32) * STAIR_DROP;
            let y = top - STAIR_DROP;
            // Deeper steps are darker, as if the light gives out.
            let light = 1.0 - 0.18 * k as f32;
            self.stairs.quad_uv(
                [
                    at(a0, -0.5, y),
                    at(a0, 0.5, y),
                    at(a1, 0.5, y),
                    at(a1, -0.5, y),
                ],
                [[0., v1], [1., v1], [1., v0], [0., v0]],
                light,
            );
            self.stairs.quad_uv(
                [
                    at(a0, -0.5, y),
                    at(a0, 0.5, y),
                    at(a0, 0.5, top),
                    at(a0, -0.5, top),
                ],
                [[0., v1], [1., v1], [1., v1 - 0.1], [0., v1 - 0.1]],
                light * 0.7,
            );
            let side = light;
            self.walls[0].quad_uv(
                [
                    at(a0, -0.5, y),
                    at(a1, -0.5, y),
                    at(a1, -0.5, 0.),
                    at(a0, -0.5, 0.),
                ],
                [[v0, -y], [v1, -y], [v1, 0.], [v0, 0.]],
                side,
            );
            self.walls[0].quad_uv(
                [
                    at(a1, 0.5, y),
                    at(a0, 0.5, y),
                    at(a0, 0.5, 0.),
                    at(a1, 0.5, 0.),
                ],
                [[v1, -y], [v0, -y], [v0, 0.], [v1, 0.]],
                side,
            );
        }
        let bottom = -n * STAIR_DROP;
        self.stairs.quad(
            [
                at(0.5, -0.5, bottom),
                at(0.5, 0.5, bottom),
                at(0.5, 0.5, 0.),
                at(0.5, -0.5, 0.),
            ],
            0.0,
        );
    }
}

/// Unit world vector for a grid direction (north is -Z).
pub fn grid_dir(dir: Facing) -> Vec3 {
    let (dx, dy) = dir.delta();
    Vec3::new(dx as f32, 0.0, dy as f32)
}

/// Center of a cell at floor height.
pub fn cell_center(p: GridPos) -> Vec3 {
    Vec3::new(p.x as f32 + 0.5, 0.0, p.y as f32 + 0.5)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every quad of every wall variant, as (centre, normal).
    fn wall_quads(geo: &LevelGeometry) -> Vec<(Vec3, Vec3)> {
        geo.walls
            .iter()
            .flat_map(|b| {
                b.positions
                    .chunks(4)
                    .zip(b.normals.chunks(4))
                    .map(|(ps, ns)| {
                        let center = ps.iter().map(|p| Vec3::from(*p)).sum::<Vec3>() / 4.0;
                        (center, Vec3::from(ns[0]))
                    })
            })
            .collect()
    }

    #[test]
    fn wall_faces_point_into_their_cell() {
        let mut map = Map::filled(3, 3, Tile::Wall);
        map.set(GridPos::new(1, 1), Tile::Floor);
        for seed in 0..20 {
            let geo = LevelGeometry::build(&map, Facing::North, seed);
            let quads = wall_quads(&geo);
            assert_eq!(quads.len(), 4);
            let center = cell_center(GridPos::new(1, 1)) + Vec3::Y * 0.5;
            for (pos, n) in quads {
                assert!(n.dot(center - pos) > 0.0, "a wall faces outwards");
            }
            let floor = geo.floors.iter().find(|b| !b.is_empty()).unwrap();
            assert_eq!(floor.normals[0], [0.0, 1.0, 0.0]);
            assert_eq!(geo.ceilings.normals[0], [0.0, -1.0, 0.0]);
        }
    }

    #[test]
    fn dressing_varies_but_stays_put() {
        let mut map = Map::filled(12, 12, Tile::Wall);
        for y in 1..11 {
            for x in 1..11 {
                map.set(GridPos::new(x, y), Tile::Floor);
            }
        }
        let a = LevelGeometry::build(&map, Facing::North, 7);
        let b = LevelGeometry::build(&map, Facing::North, 7);
        let counts = |g: &LevelGeometry| g.walls.each_ref().map(|w| w.indices.len());
        assert_eq!(counts(&a), counts(&b), "same seed, same walls");
        assert!(
            counts(&a).iter().all(|&n| n > 0),
            "every wall variant appears"
        );
        assert!(!a.vines.is_empty() && !a.ferns.is_empty());
        // Vines sit just in front of their wall, facing into the room.
        for (p, n) in a.vines.positions.iter().zip(&a.vines.normals).step_by(4) {
            let (p, n) = (Vec3::from(*p), Vec3::from(*n));
            let wall_plane = if n.x != 0.0 { p.x.round() } else { p.z.round() };
            let offset = if n.x != 0.0 {
                p.x - wall_plane
            } else {
                p.z - wall_plane
            };
            let along = if n.x != 0.0 { n.x } else { n.z };
            assert!((offset.abs() - DECAL_OFFSET).abs() < 1e-4);
            assert_eq!(offset.signum(), along.signum(), "vines behind their wall");
        }
        // Ferns stand on the floor, inside the room.
        for p in &a.ferns.positions {
            assert!(p[1] >= 0.0 && p[1] <= FERN_SIZE + 1e-5);
            assert!(p[0] > 1.0 && p[0] < 11.0 && p[2] > 1.0 && p[2] < 11.0);
        }
    }

    #[test]
    fn tangents_follow_the_uvs() {
        // A wall facing +Z: U runs along +X and the texture's top is up.
        let corners = [
            Vec3::new(0., 0., 0.),
            Vec3::new(1., 0., 0.),
            Vec3::new(1., 1., 0.),
            Vec3::new(0., 1., 0.),
        ];
        let n = Vec3::Z;
        let bitangent = |t: Vec4| t.w * n.cross(t.truncate());
        let t = quad_tangent(corners, UV, n);
        assert_eq!(t, Vec4::new(1., 0., 0., 1.));
        assert!(
            bitangent(t).dot(Vec3::Y) > 0.99,
            "bitangent must point up the texture"
        );
        // Mirrored UVs (as on one pit wall) flip the tangent, not the bitangent.
        let mirrored = [[1., 1.], [0., 1.], [0., 0.], [1., 0.]];
        let t = quad_tangent(corners, mirrored, n);
        assert_eq!(t.truncate(), Vec3::NEG_X);
        assert!(bitangent(t).dot(Vec3::Y) > 0.99);
    }

    #[test]
    fn torches_are_few_and_spread_out() {
        let mut map = Map::filled(12, 12, Tile::Wall);
        for y in 1..11 {
            for x in 1..11 {
                map.set(GridPos::new(x, y), Tile::Floor);
            }
        }
        let mut total = 0;
        for seed in 0..40 {
            let geo = LevelGeometry::build(&map, Facing::North, seed);
            assert!(geo.torches.len() <= MAX_TORCHES);
            total += geo.torches.len();
            let cells: Vec<GridPos> = geo
                .torches
                .iter()
                .map(|t| {
                    let p = t.flame + t.facing * 0.2;
                    GridPos::new(p.x.floor() as i32, p.z.floor() as i32)
                })
                .collect();
            for (i, a) in cells.iter().enumerate() {
                assert!(map.is_walkable(*a), "seed {seed}: torch outside the room");
                for b in &cells[i + 1..] {
                    assert!(
                        a.manhattan(*b) >= TORCH_SPACING,
                        "seed {seed}: torches too close"
                    );
                }
            }
        }
        assert!(
            total >= 40,
            "rooms should usually get torches ({total} in 40 floors)"
        );
    }

    #[test]
    fn stairs_lead_away_and_replace_the_floor() {
        // A dead end entered from the south: the steps go down northwards.
        let mut map = Map::filled(3, 4, Tile::Wall);
        map.set(GridPos::new(1, 1), Tile::StairsDown);
        map.set(GridPos::new(1, 2), Tile::Floor);
        let geo = LevelGeometry::build(&map, Facing::North, 3);
        let floors: usize = geo.floors.iter().map(|b| b.indices.len()).sum();
        assert_eq!(floors, 6, "only the corridor cell has a floor");
        // Treads face up; risers face back towards the entrance (+Z).
        assert_eq!(geo.stairs.normals[0], [0.0, 1.0, 0.0]);
        assert_eq!(geo.stairs.normals[4], [0.0, 0.0, 1.0]);
        let lowest = geo
            .stairs
            .positions
            .iter()
            .map(|p| p[1])
            .fold(0.0, f32::min);
        assert!((lowest + STAIR_STEPS as f32 * STAIR_DROP).abs() < 1e-5);
        assert_eq!(geo.arch.indices.len(), 6, "an arch on the north wall");
        assert_eq!(geo.arch.normals[0], [0.0, 0.0, 1.0]);
        // Every wall (pit walls included) faces into its own cell.
        for (center, n) in wall_quads(&geo) {
            let cell = cell_center(GridPos::new(1, if center.z > 2.0 { 2 } else { 1 }));
            assert!(n.dot(cell - center) > 0.0, "wall at {center} faces out");
        }
    }
}
