//! Dream floors, drawn from the Dreamscape modular kits
//! (`art/kits/dreamscape-modular-v1`); `zrpg_core::dream` has their maps.
//!
//! The Mindglass Sanctum is laid out by `tools/make_dreamscape.py` into
//! `assets/dream/mindglass-sanctum/` (its placed modules, four role textures
//! and crystal lights). The patchworks come from `tools/make_patchwork.py`:
//! one module library, a pool of themes with their textures and looks
//! (`dream_pool.rs`), and per floor the placed instances, the exhibits'
//! lights, and which theme every cell is in.
//!
//! Materials follow the kit's contract: the four roles bind by name to its
//! diffuse candidates (no normal or roughness maps are supplied, so none are
//! invented); wall and ground repeat every 2 m, structure and detail panels
//! clamp. Crisp pixels up close, mipmaps far away. The detail role glows
//! faintly, and the lights breathe slowly.
//!
//! Crossing from one room of a patchwork into the next, the haze, the sky
//! colour and the ambient light blend into the new room's ([`thresholds`]).

use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use zrpg_core::dream::{DreamId, SANCTUM};

use crate::atmosphere::{GradedMaterial, Look, untonemapped};
use crate::dream_pool::{FLOORS, MODULES, THEMES};
use crate::kit_level::{self, Library, MeshData, Reader, SCALE};
use crate::view::PlayerCamera;
use crate::{Run, textures};

const LEVEL: &[u8] = include_bytes!("../assets/dream/mindglass-sanctum/level.bin");
/// The sanctum's four roles, in the order `tools/make_dreamscape.py`
/// numbers them.
const SANCTUM_ROLES: [&[u8]; 4] = [
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
/// The lights: how far they reach, and how they breathe.
const GLOW_RANGE: f32 = 6.5;
const BREATH_HZ: f32 = 0.23;
const BREATH_DEPTH: f32 = 0.18;
/// How quickly a threshold's change of air settles (per second).
const THRESHOLD_RATE: f32 = 2.5;

#[derive(Resource)]
pub struct DreamAssets {
    sanctum: Vec<(Handle<Mesh>, Handle<GradedMaterial>)>,
    /// Where the sanctum's crystal lights are (game units), their colour
    /// and lumens.
    sanctum_glows: Vec<(Vec3, Color, f32)>,
    library: Library,
    /// Four per pool theme, in role order.
    theme_materials: Vec<Handle<GradedMaterial>>,
}

/// A breathing light.
#[derive(Component)]
pub struct Glow {
    lumens: f32,
    phase: f32,
}

/// Which pool theme each cell of the current patchwork is in (255 for
/// rock), for [`thresholds`].
#[derive(Resource)]
pub struct DreamCells {
    width: i32,
    height: i32,
    themes: Vec<u8>,
}

impl DreamCells {
    fn theme_at(&self, at: Vec3) -> Option<usize> {
        let (x, z) = (at.x.floor() as i32, at.z.floor() as i32);
        if x < 0 || z < 0 || x >= self.width || z >= self.height {
            return None;
        }
        let t = self.themes[(z * self.width + x) as usize];
        (t != 255).then_some(t as usize)
    }
}

/// A pool theme's look: its haze, ambient light and distance tint, as
/// `make_patchwork.py` derived them from its textures.
pub fn theme_look(theme: usize) -> Look {
    let t = &THEMES[theme];
    let mut look = Look::SANCTUM;
    look.sky = Color::srgb(t.fog[0], t.fog[1], t.fog[2]);
    look.ambient_color = Color::linear_rgb(t.ambient[0], t.ambient[1], t.ambient[2]);
    look.set_tint(Vec3::from_array(t.tint));
    look
}

fn role_material(
    png: &[u8],
    role: usize,
    look: &Look,
    images: &mut Assets<Image>,
    materials: &mut Assets<GradedMaterial>,
) -> Handle<GradedMaterial> {
    let mut image = textures::with_mipmaps(textures::embedded(png));
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
}

fn read_glows(r: &mut Reader) -> Vec<(Vec3, Color, f32)> {
    (0..r.u32())
        .map(|_| {
            let [v] = r.f32s::<7>(1).try_into().unwrap();
            (
                Vec3::new(v[0], v[1], v[2]) * SCALE,
                Color::linear_rgb(v[3], v[4], v[5]),
                v[6],
            )
        })
        .collect()
}

pub fn load(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<GradedMaterial>>,
) {
    let sanctum_mats: Vec<Handle<GradedMaterial>> = (0..4)
        .map(|role| {
            role_material(
                SANCTUM_ROLES[role],
                role,
                &Look::SANCTUM,
                &mut images,
                &mut materials,
            )
        })
        .collect();
    let (merged, mut r) = kit_level::parse(LEVEL, 4, |slot, _| slot);
    let sanctum_glows = read_glows(&mut r);
    assert!(r.is_done(), "sanctum level.bin has trailing data");
    let theme_materials = (0..THEMES.len())
        .flat_map(|t| (0..4).map(move |role| (t, role)))
        .map(|(t, role)| {
            role_material(
                THEMES[t].textures[role],
                role,
                &theme_look(t),
                &mut images,
                &mut materials,
            )
        })
        .collect();
    commands.insert_resource(DreamAssets {
        sanctum: merged
            .into_iter()
            .map(MeshData::into_mesh)
            .zip(sanctum_mats)
            .map(|(mesh, mat)| (meshes.add(mesh), mat))
            .collect(),
        sanctum_glows,
        library: Library::parse(MODULES),
        theme_materials,
    });
}

/// A dream floor ready to spawn.
pub struct Built {
    parts: Vec<(Handle<Mesh>, Handle<GradedMaterial>)>,
    glows: Vec<(Vec3, Color, f32)>,
    /// For a patchwork: which theme each cell is in.
    pub cells: Option<DreamCells>,
}

/// Build dream `id`: the sanctum as loaded, or a patchwork placed now.
pub fn build(id: DreamId, assets: &DreamAssets, meshes: &mut Assets<Mesh>) -> Built {
    if id == SANCTUM {
        return Built {
            parts: assets.sanctum.clone(),
            glows: assets.sanctum_glows.clone(),
            cells: None,
        };
    }
    let (merged, mut r) = assets.library.place(FLOORS[id as usize - 1], THEMES.len());
    let glows = read_glows(&mut r);
    let (width, height) = (r.u32() as i32, r.u32() as i32);
    let themes = r.bytes((width * height) as usize).to_vec();
    assert!(r.is_done(), "dream floor has trailing data");
    let mut seen: Vec<u8> = themes.iter().copied().filter(|&t| t != 255).collect();
    seen.sort();
    seen.dedup();
    let names: Vec<&str> = seen.iter().map(|&t| THEMES[t as usize].id).collect();
    info!("dream {id}: a patchwork of {}", names.join(", "));
    let parts = merged
        .into_iter()
        .enumerate()
        .filter(|(_, m)| !m.indices.is_empty())
        .map(|(slot, m)| {
            (
                meshes.add(m.into_mesh()),
                assets.theme_materials[slot].clone(),
            )
        })
        .collect();
    Built {
        parts,
        glows,
        cells: Some(DreamCells {
            width,
            height,
            themes,
        }),
    }
}

/// A dream's geometry and lights, as children of the level.
pub fn spawn(parent: &mut ChildSpawnerCommands, built: &Built) {
    for (mesh, material) in &built.parts {
        parent.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material.clone())));
    }
    for (i, &(at, color, lumens)) in built.glows.iter().enumerate() {
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

/// The lights swell and fade, slowly and out of step.
pub fn breathe(time: Res<Time>, mut lights: Query<(&Glow, &mut PointLight)>) {
    let t = time.elapsed_secs();
    for (glow, mut light) in &mut lights {
        let breath = (t * BREATH_HZ * std::f32::consts::TAU + glow.phase).sin();
        light.intensity = glow.lumens * (1.0 + BREATH_DEPTH * breath);
    }
}

/// The air a patchwork's thresholds have blended so far: which floor, the
/// sky colour and the ambient light.
type Air = ((u64, u32), LinearRgba, LinearRgba);

/// Crossing between rooms of a patchwork, the air changes: the haze, the
/// sky behind it and the ambient light ease toward the room you are in.
pub fn thresholds(
    time: Res<Time>,
    run: Res<Run>,
    cells: Option<Res<DreamCells>>,
    mut cams: Query<
        (&Transform, &mut Camera, &mut DistanceFog, &mut AmbientLight),
        With<PlayerCamera>,
    >,
    mut air: Local<Option<Air>>,
) {
    let Some(cells) =
        cells.filter(|_| matches!(run.floor.place, zrpg_core::Place::Dream(id) if id != SANCTUM))
    else {
        *air = None;
        return;
    };
    let Ok((at, mut camera, mut fog, mut ambient)) = cams.single_mut() else {
        return;
    };
    let Some(theme) = cells.theme_at(at.translation) else {
        return;
    };
    let look = theme_look(theme);
    let (sky, light) = (look.sky.to_linear(), look.ambient_color.to_linear());
    let floor = (run.run_seed, run.depth);
    let (sky, light) = match *air {
        // A new floor starts in its first room's air.
        Some((f, s, l)) if f == floor => {
            let k = 1.0 - (-THRESHOLD_RATE * time.delta_secs()).exp();
            (s.mix(&sky, k), l.mix(&light, k))
        }
        _ => (sky, light),
    };
    *air = Some((floor, sky, light));
    let sky_color = Color::LinearRgba(sky);
    camera.clear_color = ClearColorConfig::Custom(sky_color);
    fog.color = untonemapped(sky_color);
    ambient.color = Color::LinearRgba(light);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The sanctum lands on the game grid: the 15 x 16 cell layout, its
    /// coffered ceiling up at 4.26 m (halved), its floor slabs' tops at 0,
    /// and its lights in the cells where the crystals stand.
    #[test]
    fn the_sanctum_lands_on_the_game_grid() {
        let (meshes, mut r) = kit_level::parse(LEVEL, 4, |slot, _| slot);
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
        let floor = zrpg_core::dream::sanctum();
        let glows = read_glows(&mut r);
        assert_eq!(glows.len(), 7);
        for (at, _, _) in glows {
            let cell = zrpg_core::GridPos::new(at.x as i32, at.z as i32);
            assert!(
                !floor.map.is_walkable(cell),
                "a light in the lane at {cell:?}"
            );
        }
        assert!(r.is_done());
    }

    /// Every patchwork floor places cleanly and agrees with its map: the
    /// cell grids match, every walkable cell is in a theme, and every room
    /// is lit.
    #[test]
    fn patchworks_agree_with_their_maps() {
        let library = Library::parse(MODULES);
        for (k, bytes) in FLOORS.iter().enumerate() {
            let (meshes, mut r) = library.place(bytes, THEMES.len());
            let triangles: usize = meshes.iter().map(|m| m.indices.len() / 3).sum();
            // `make_patchwork.py`'s budget, for modest hardware.
            assert!(triangles <= 45_000, "floor {k}: {triangles} triangles");
            let glows = read_glows(&mut r);
            assert!(!glows.is_empty(), "floor {k} has no light");
            let (w, h) = (r.u32() as i32, r.u32() as i32);
            let themes = r.bytes((w * h) as usize).to_vec();
            assert!(r.is_done());
            let floor = zrpg_core::dream::floor(k as DreamId + 1);
            assert_eq!((floor.map.width(), floor.map.height()), (w, h), "floor {k}");
            for p in floor.map.positions().filter(|&p| floor.map.is_walkable(p)) {
                let t = themes[(p.y * w + p.x) as usize];
                assert!((t as usize) < THEMES.len(), "floor {k}: {p:?} has no theme");
            }
        }
    }
}
