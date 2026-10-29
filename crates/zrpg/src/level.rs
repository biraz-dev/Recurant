//! Turns a `zrpg_core::Map` into geometry: one mesh per material, so a whole
//! floor costs about a dozen draw calls. The ruins (see `ruins.rs`) are
//! built from their asset kit instead; only their way down comes from here.
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
use zrpg_core::{Facing, GridPos, Map, Place, Theme, Tile};

use crate::Run;
use crate::atmosphere::{GradedMaterial, graded};
use crate::dreamscape::{self, DreamAssets};
use crate::lighting::{self, TorchAssets};
use crate::ruins::{self, RuinsAssets};
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

/// How a floor is dressed: the chance of each wall and floor variant, of
/// vines, ferns and torches. Each stage has its own mix.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dressing {
    carved: f32,
    cracked: f32,
    mossy_wall: f32,
    mossy_floor: f32,
    vines: f32,
    fern: f32,
    torch: f32,
}

impl Dressing {
    /// The original dungeon's mix.
    const CLASSIC: Dressing = Dressing {
        carved: CARVED,
        cracked: CRACKED,
        mossy_wall: MOSSY_WALL,
        mossy_floor: MOSSY_FLOOR,
        vines: VINES,
        fern: FERN,
        torch: TORCH,
    };

    pub fn for_theme(theme: Theme) -> Dressing {
        let c = Dressing::CLASSIC;
        match theme {
            // Damp and green.
            Theme::Crypt => Dressing {
                mossy_wall: 0.35,
                mossy_floor: 0.45,
                vines: 0.35,
                ..c
            },
            // Old, split stone; little grows.
            Theme::Catacombs => Dressing {
                cracked: 0.4,
                mossy_wall: 0.05,
                mossy_floor: 0.1,
                vines: 0.08,
                fern: 0.05,
                ..c
            },
            // Carved, cared-for walls, well lit.
            Theme::Tombs => Dressing {
                carved: 0.3,
                cracked: 0.06,
                mossy_wall: 0.04,
                mossy_floor: 0.08,
                vines: 0.05,
                fern: 0.04,
                torch: 0.16,
            },
            // Scorched: cracked, bare, many fires.
            Theme::EmberDeep => Dressing {
                carved: 0.02,
                cracked: 0.45,
                mossy_wall: 0.0,
                mossy_floor: 0.0,
                vines: 0.0,
                fern: 0.0,
                torch: 0.18,
            },
            // Overgrown and dim.
            Theme::Haunted => Dressing {
                mossy_wall: 0.3,
                vines: 0.45,
                fern: 0.3,
                torch: 0.06,
                ..c
            },
            // A fortress: carved and cracked stone, torches on the walls.
            Theme::FallenKeep => Dressing {
                carved: 0.15,
                cracked: 0.25,
                mossy_wall: 0.08,
                vines: 0.12,
                torch: 0.15,
                ..c
            },
        }
    }
}
const TORCH_SPACING: i32 = 4;
/// The sconce (bracket and torch) on the wall: size and height of its foot.
const SCONCE_SIZE: Vec2 = Vec2::new(0.2, 0.3);
const SCONCE_BOTTOM: f32 = 0.42;

/// Trim profiles, as (distance out from the wall, height) from the bottom up:
/// a chamfered skirting block where wall meets floor...
const BASEBOARD: [Vec2; 4] = [
    Vec2::new(0.035, 0.0),
    Vec2::new(0.035, 0.065),
    Vec2::new(0.02, 0.085),
    Vec2::new(0.0, 0.085),
];
/// ...and a stepped cornice with a cove where wall meets ceiling.
const CROWN: [Vec2; 5] = [
    Vec2::new(0.0, 0.86),
    Vec2::new(0.022, 0.86),
    Vec2::new(0.022, 0.9),
    Vec2::new(0.05, 0.95),
    Vec2::new(0.05, 1.0),
];

/// How a strip of trim ends at the edge of its wall.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TrimEnd {
    /// The wall carries on into the next cell, or the trim stops against a
    /// wall that has none.
    Flush,
    /// Outside corner: run on, to meet the other wall's trim in a mitre.
    Extend,
    /// Inside corner: stop short, to meet the other wall's trim in a mitre.
    Shorten,
}

/// A wall torch: where its flame burns and which way its wall faces.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Torch {
    pub flame: Vec3,
    pub facing: Vec3,
}

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GroundHeights>()
            .add_systems(Startup, (load_materials, ruins::load, dreamscape::load))
            .add_systems(
                Update,
                (
                    rebuild_level,
                    (
                        ruins::animate_water,
                        ruins::follow_sky,
                        dreamscape::breathe,
                        dreamscape::thresholds.after(crate::atmosphere::apply_look),
                    )
                        .in_set(crate::Frame::Present),
                ),
            );
    }
}

#[derive(Resource)]
struct LevelMaterials {
    walls: [Handle<GradedMaterial>; WALL_KINDS],
    floors: [Handle<GradedMaterial>; FLOOR_KINDS],
    ceiling: Handle<GradedMaterial>,
    stairs: Handle<GradedMaterial>,
    arch: Handle<GradedMaterial>,
    trim: Handle<GradedMaterial>,
    vines: Handle<GradedMaterial>,
    fern: Handle<GradedMaterial>,
    sconce: Handle<GradedMaterial>,
}

/// How high the ground is anywhere on the current floor, for the camera and
/// enemies: flat in the dungeon; in the ruins, eased between cell centres so
/// that walking into the pool or up its stairs is a slope, not a jump.
#[derive(Resource, Default)]
pub struct GroundHeights {
    width: i32,
    depth: i32,
    /// Height at each cell's centre; empty means flat at 0.
    cells: Vec<f32>,
}

impl GroundHeights {
    fn new(map: &Map, surface: impl Fn(GridPos) -> f32) -> Self {
        Self {
            width: map.width(),
            depth: map.height(),
            cells: map.positions().map(surface).collect(),
        }
    }

    pub fn at(&self, p: Vec3) -> f32 {
        if self.cells.is_empty() {
            return 0.0;
        }
        let (u, v) = (p.x - 0.5, p.z - 0.5);
        let (x, z) = (u.floor() as i32, v.floor() as i32);
        let (fu, fv) = (u - u.floor(), v - v.floor());
        let h = |x: i32, z: i32| {
            if x < 0 || z < 0 || x >= self.width || z >= self.depth {
                0.0
            } else {
                self.cells[(z * self.width + x) as usize]
            }
        };
        let near = h(x, z) + (h(x + 1, z) - h(x, z)) * fu;
        let far = h(x, z + 1) + (h(x + 1, z + 1) - h(x, z + 1)) * fu;
        near + (far - near) * fv
    }
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
    mut materials: ResMut<Assets<GradedMaterial>>,
) {
    // Cutouts (vines, ferns, sconces) are alpha-masked and visible from both
    // sides. The ORM map serves as both occlusion and roughness (glTF packing).
    let mut make = |maps: Maps, cutout: bool| {
        let orm = images.add(textures::embedded_linear(maps.orm));
        materials.add(graded(StandardMaterial {
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
        }))
    };
    commands.insert_resource(LevelMaterials {
        walls: env::WALLS.map(|maps| make(maps, false)),
        floors: env::FLOORS.map(|maps| make(maps, false)),
        ceiling: make(env::CEILING, false),
        stairs: make(env::STAIRS, false),
        arch: make(env::ARCH, false),
        trim: make(env::TRIM, false),
        vines: make(env::VINES, true),
        fern: make(env::FERN, true),
        sconce: make(env::SCONCE, true),
    });
}

#[allow(clippy::too_many_arguments)]
fn rebuild_level(
    mut commands: Commands,
    run: Res<Run>,
    roots: Query<(Entity, &LevelRoot)>,
    mats: Res<LevelMaterials>,
    torch_assets: Res<TorchAssets>,
    ruins_assets: Res<RuinsAssets>,
    dream_assets: Res<DreamAssets>,
    mut ground: ResMut<GroundHeights>,
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
    let place = run.floor.place;
    let geo = match place {
        Place::Dungeon => {
            *ground = GroundHeights::default();
            let dressing = run
                .floor
                .stage
                .map_or(Dressing::CLASSIC, |s| Dressing::for_theme(s.theme));
            LevelGeometry::dress(&run.floor.map, run.floor.stairs_dir, seed, &dressing)
        }
        Place::Ruins => {
            *ground = GroundHeights::new(&run.floor.map, ruins::surface);
            // The way down: dungeon stairs sunk into the ruin's floor.
            let mut geo = LevelGeometry::default();
            geo.add_stairs(run.floor.stairs, run.floor.stairs_dir);
            geo
        }
        Place::Dream(_) => {
            // Level floors; the way down sinks into the reliquary floor, as
            // in the ruins.
            *ground = GroundHeights::default();
            let mut geo = LevelGeometry::default();
            geo.add_stairs(run.floor.stairs, run.floor.stairs_dir);
            geo
        }
    };
    let mut dream = match place {
        Place::Dream(id) => Some(dreamscape::build(id, &dream_assets, &mut meshes)),
        _ => None,
    };
    // A patchwork tells the thresholds which room every cell is in.
    match dream.as_mut().and_then(|d| d.cells.take()) {
        Some(cells) => commands.insert_resource(cells),
        None => commands.remove_resource::<dreamscape::DreamCells>(),
    }
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
            match (&place, &dream) {
                (Place::Ruins, _) => ruins::spawn(parent, &ruins_assets),
                (Place::Dream(_), Some(built)) => dreamscape::spawn(parent, built),
                _ => {}
            }
            let parts = geo
                .walls
                .into_iter()
                .zip(&mats.walls)
                .chain(geo.floors.into_iter().zip(&mats.floors))
                .chain([
                    (geo.ceilings, &mats.ceiling),
                    (geo.stairs, &mats.stairs),
                    (geo.arch, &mats.arch),
                    (geo.trims, &mats.trim),
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
    /// Baseboards and crown molding.
    trims: QuadBuilder,
    vines: QuadBuilder,
    ferns: QuadBuilder,
    sconces: QuadBuilder,
    torches: Vec<Torch>,
}

impl LevelGeometry {
    #[cfg(test)]
    fn build(map: &Map, stairs_dir: Facing, seed: u64) -> Self {
        Self::dress(map, stairs_dir, seed, &Dressing::CLASSIC)
    }

    fn dress(map: &Map, stairs_dir: Facing, seed: u64, d: &Dressing) -> Self {
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
                let kind = usize::from(roll(seed, p, 1) < d.mossy_floor);
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
                // Crown molding runs along every wall; baseboards along every
                // wall but the arch the stairs lead under.
                let has_baseboard =
                    |q: GridPos, d: Facing| !(map.get(q) == Tile::StairsDown && d == stairs_dir);
                let crown_ends = trim_ends(map, p, dir, |_, _| true);
                geo.add_trim(p, dir, &CROWN, crown_ends, crown_shade);
                if has_baseboard(p, dir) {
                    let ends = trim_ends(map, p, dir, has_baseboard);
                    geo.add_trim(p, dir, &BASEBOARD, ends, baseboard_shade);
                }
                if stairs && dir == stairs_dir {
                    let (foot, top) = (light * WALL_FOOT_SHADE, light * WALL_TOP_SHADE);
                    geo.arch.quad_shaded(wall, UV, [foot, foot, top, top]);
                    continue;
                }
                let salt = 10 + dir as u64;
                let kind = match roll(seed, p, salt) {
                    r if r < d.carved => 3,
                    r if r < d.carved + d.cracked => 2,
                    r if r < d.carved + d.cracked + d.mossy_wall => 1,
                    _ => 0,
                };
                let foot = light * WALL_FOOT_SHADE;
                let top = light * WALL_TOP_SHADE;
                geo.walls[kind].quad_shaded(wall, UV, [foot, foot, top, top]);
                let t = roll(seed, p, salt + 30);
                if t < d.torch {
                    torch_spots.push((t, p, dir));
                }
                if roll(seed, p, salt + 10) < d.vines {
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
            if !stairs && roll(seed, p, 30) < d.fern {
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

    /// Extrude a trim `profile` along the wall of cell `p` facing `dir`.
    fn add_trim(
        &mut self,
        p: GridPos,
        dir: Facing,
        profile: &[Vec2],
        ends: [TrimEnd; 2],
        shade: fn(f32) -> f32,
    ) {
        let (f, r) = (grid_dir(dir), grid_dir(dir.right()));
        let wall = cell_center(p) + f * 0.5;
        // Where a profile point ends along the wall (-0.5 left .. 0.5 right).
        // Mitres move each point by its own distance from the wall.
        let reach = |pt: Vec2, end: TrimEnd, side: f32| {
            side * match end {
                TrimEnd::Flush => 0.5,
                TrimEnd::Extend => 0.5 + pt.x,
                TrimEnd::Shorten => 0.5 - pt.x,
            }
        };
        let at = |pt: Vec2, s: f32| wall - f * pt.x + Vec3::Y * pt.y + r * s;
        let mut arc = 0.0;
        for seg in profile.windows(2) {
            let (a, b) = (seg[0], seg[1]);
            let (al, ar) = (reach(a, ends[0], -1.0), reach(a, ends[1], 1.0));
            let (bl, br) = (reach(b, ends[0], -1.0), reach(b, ends[1], 1.0));
            // Texture: U along the wall, V along the profile, 1 unit = 1 tile.
            let len = (b - a).length();
            let (va, vb) = (0.5 - arc, 0.5 - arc - len);
            self.trims.quad_shaded(
                [at(a, al), at(a, ar), at(b, br), at(b, bl)],
                [
                    [al + 0.5, va],
                    [ar + 0.5, va],
                    [br + 0.5, vb],
                    [bl + 0.5, vb],
                ],
                [shade(a.y), shade(a.y), shade(b.y), shade(b.y)],
            );
            arc += len;
        }
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

/// How the trim on the wall of `p` facing `dir` should end on each side
/// (left, right). `has_trim` says whether a wall face carries the same trim.
fn trim_ends(
    map: &Map,
    p: GridPos,
    dir: Facing,
    has_trim: impl Fn(GridPos, Facing) -> bool,
) -> [TrimEnd; 2] {
    [dir.left(), dir.right()].map(|side| {
        let beside = p + side;
        if !map.is_walkable(beside) {
            // Inside corner: the wall on that side meets this one.
            if has_trim(p, side) {
                TrimEnd::Shorten
            } else {
                TrimEnd::Flush
            }
        } else if map.is_walkable(beside + dir) {
            // Outside corner: this wall ends and turns away.
            TrimEnd::Extend
        } else {
            TrimEnd::Flush
        }
    })
}

/// Baked contact shadow on trim: dark where the baseboard meets the floor
/// and under the cornice.
fn baseboard_shade(y: f32) -> f32 {
    if y < 0.01 { 0.72 } else { 1.0 }
}

fn crown_shade(y: f32) -> f32 {
    if y < 0.87 {
        0.7
    } else if y > 0.99 {
        0.8
    } else {
        0.92
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
    fn trim_mitres_inside_and_outside_corners() {
        // An L of three cells: (1,1) and (2,1) along the top, (1,2) below.
        //   # # # #
        //   # . . #
        //   # . # #
        //   # # # #
        let mut map = Map::filled(4, 4, Tile::Wall);
        for (x, y) in [(1, 1), (2, 1), (1, 2)] {
            map.set(GridPos::new(x, y), Tile::Floor);
        }
        let any = |_: GridPos, _: Facing| true;
        use TrimEnd::*;
        // North wall of (1,1): inside corner with the west wall on its left;
        // on its right the wall carries on over (2,1).
        assert_eq!(
            trim_ends(&map, GridPos::new(1, 1), Facing::North, any),
            [Shorten, Flush]
        );
        // South wall of (2,1): its left (east) is solid, an inside corner; on
        // its right (west) the cell (1,1) is open and so is (1,2) below it:
        // the wall block (2,2) has an outside corner there.
        assert_eq!(
            trim_ends(&map, GridPos::new(2, 1), Facing::South, any),
            [Shorten, Extend]
        );
        // A wall without trim (like the stairs arch) gets a flush end instead.
        let none_west = |_: GridPos, d: Facing| d != Facing::West;
        assert_eq!(
            trim_ends(&map, GridPos::new(1, 1), Facing::North, none_west),
            [Flush, Flush]
        );
    }

    #[test]
    fn trim_stays_in_the_room_and_faces_out_of_the_wall() {
        let mut map = Map::filled(3, 3, Tile::Wall);
        map.set(GridPos::new(1, 1), Tile::Floor);
        let geo = LevelGeometry::build(&map, Facing::North, 1);
        assert!(!geo.trims.is_empty());
        let center = cell_center(GridPos::new(1, 1));
        for (ps, ns) in geo
            .trims
            .positions
            .chunks(4)
            .zip(geo.trims.normals.chunks(4))
        {
            let n = Vec3::from(ns[0]);
            for p in ps {
                // Enclosed cell: every mitre is cut short, nothing pokes out.
                assert!(
                    (1.0..=2.0).contains(&p[0]) && (1.0..=2.0).contains(&p[2]),
                    "{p:?}"
                );
            }
            // Faces point into the room, or straight up or down.
            let mid = ps.iter().map(|p| Vec3::from(*p)).sum::<Vec3>() / 4.0;
            let inward = (center - mid).with_y(0.0);
            assert!(
                n.y.abs() > 0.99 || n.dot(inward) > 0.0,
                "trim face turned into the wall"
            );
        }
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
