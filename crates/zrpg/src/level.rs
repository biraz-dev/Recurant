//! Turns a `zrpg_core::Map` into geometry: one mesh per material, so a whole
//! floor costs four draw calls.
//!
//! World layout: grid cell `(x, y)` spans `x..x+1` on X and `y..y+1` on Z,
//! floor at height 0, ceiling at `WALL_HEIGHT`.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use zrpg_core::{Facing, GridPos, Map, Tile};

use crate::Run;
use crate::textures;

pub const WALL_HEIGHT: f32 = 1.0;

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_materials)
            .add_systems(Update, rebuild_level);
    }
}

#[derive(Resource)]
struct LevelMaterials {
    wall: Handle<StandardMaterial>,
    floor: Handle<StandardMaterial>,
    ceiling: Handle<StandardMaterial>,
    stairs: Handle<StandardMaterial>,
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
    let mut make = |image: Image| {
        materials.add(StandardMaterial {
            base_color_texture: Some(images.add(image)),
            unlit: true,
            ..default()
        })
    };
    commands.insert_resource(LevelMaterials {
        wall: make(textures::wall()),
        floor: make(textures::floor()),
        ceiling: make(textures::ceiling()),
        stairs: make(textures::stairs()),
    });
}

fn rebuild_level(
    mut commands: Commands,
    run: Res<Run>,
    roots: Query<(Entity, &LevelRoot)>,
    mats: Res<LevelMaterials>,
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

    let geo = LevelGeometry::build(&run.floor.map);
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
            for (builder, material) in [
                (geo.walls, &mats.wall),
                (geo.floors, &mats.floor),
                (geo.ceilings, &mats.ceiling),
                (geo.stairs, &mats.stairs),
            ] {
                if !builder.is_empty() {
                    parent.spawn((
                        Mesh3d(meshes.add(builder.into_mesh())),
                        MeshMaterial3d(material.clone()),
                    ));
                }
            }
        });
}

/// Classic crawler shading: faces lit by direction instead of by lights.
fn face_brightness(normal_dir: Facing) -> f32 {
    match normal_dir {
        Facing::North | Facing::South => 1.0,
        Facing::East | Facing::West => 0.78,
    }
}

#[derive(Default)]
struct QuadBuilder {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
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
        let normal = (corners[1] - corners[0])
            .cross(corners[2] - corners[0])
            .normalize();
        let base = self.positions.len() as u32;
        for (corner, uv) in corners.iter().zip([[0., 1.], [1., 1.], [1., 0.], [0., 0.]]) {
            self.positions.push(corner.to_array());
            self.normals.push(normal.to_array());
            self.uvs.push(uv);
            self.colors.push([brightness, brightness, brightness, 1.0]);
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
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.colors)
        .with_inserted_indices(Indices::U32(self.indices))
    }
}

#[derive(Default)]
struct LevelGeometry {
    walls: QuadBuilder,
    floors: QuadBuilder,
    ceilings: QuadBuilder,
    stairs: QuadBuilder,
}

impl LevelGeometry {
    fn build(map: &Map) -> Self {
        let mut geo = LevelGeometry::default();
        let h = WALL_HEIGHT;
        for p in map.positions().filter(|&p| map.is_walkable(p)) {
            let (x0, z0) = (p.x as f32, p.y as f32);
            let (x1, z1) = (x0 + 1.0, z0 + 1.0);

            let floor = [
                Vec3::new(x0, 0., z1),
                Vec3::new(x1, 0., z1),
                Vec3::new(x1, 0., z0),
                Vec3::new(x0, 0., z0),
            ];
            match map.get(p) {
                Tile::StairsDown => geo.stairs.quad(floor, 1.0),
                _ => geo.floors.quad(floor, 1.0),
            }
            geo.ceilings.quad(
                [
                    Vec3::new(x0, h, z0),
                    Vec3::new(x1, h, z0),
                    Vec3::new(x1, h, z1),
                    Vec3::new(x0, h, z1),
                ],
                1.0,
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
                geo.walls.quad(
                    [l, r, r + Vec3::Y * h, l + Vec3::Y * h],
                    face_brightness(dir),
                );
            }
        }
        geo
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

    #[test]
    fn wall_faces_point_into_their_cell() {
        let mut map = Map::filled(3, 3, Tile::Wall);
        map.set(GridPos::new(1, 1), Tile::Floor);
        let geo = LevelGeometry::build(&map);
        assert_eq!(geo.walls.indices.len(), 4 * 6);
        let center = cell_center(GridPos::new(1, 1)) + Vec3::Y * 0.5;
        for quad in 0..4 {
            let i = quad * 4;
            let pos = Vec3::from(geo.walls.positions[i]);
            let n = Vec3::from(geo.walls.normals[i]);
            assert!(n.dot(center - pos) > 0.0, "wall {quad} faces outwards");
        }
        assert_eq!(geo.floors.normals[0], [0.0, 1.0, 0.0]);
        assert_eq!(geo.ceilings.normals[0], [0.0, -1.0, 0.0]);
    }
}
