//! Something in the air: a few dozen specks drifting around you, different
//! in each place. Sunlit pollen in the ruins, dust turning in the torchlight
//! below, gilded dust in the tombs, ash in the fallen keep, embers rising in
//! the ember deep and pale wisps in the haunted halls.
//!
//! They live in a box that travels with you: one that drifts out of it comes
//! back in on the far side, so the air never empties or thickens.

use bevy::asset::RenderAssetUsages;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use zrpg_core::{Place, Theme};

use crate::view::PlayerCamera;
use crate::{Frame, Run};

/// Half the size of the box of air around you, and how far ahead of you
/// it is centred (motes behind you are wasted).
const HALF_BOX: Vec3 = Vec3::new(1.4, 0.75, 1.4);
const AHEAD: f32 = 1.0;
/// The soft dot every mote is drawn with, in pixels.
const DOT: u32 = 8;

pub struct MotesPlugin;

impl Plugin for MotesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load)
            .add_systems(Update, (refill, drift).chain().in_set(Frame::Present));
    }
}

/// How the air moves in one kind of place.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Air {
    count: usize,
    /// Linear colour (above 1 glows).
    color: [f32; 3],
    opacity: f32,
    /// World size of a mote, and how much it varies.
    size: f32,
    /// Steady drift (world units per second) and how much each mote
    /// wanders about it.
    flow: Vec3,
    wander: f32,
    /// How fast they twinkle (0: steady).
    twinkle: f32,
}

impl Air {
    fn of(place: Place, theme: Option<Theme>) -> Air {
        let dust = Air {
            count: 34,
            color: [1.4, 1.1, 0.75],
            opacity: 0.7,
            size: 0.016,
            flow: Vec3::new(0.0, -0.01, 0.0),
            wander: 0.05,
            twinkle: 1.5,
        };
        match (place, theme) {
            // Slow violet sparkles hanging in the sanctum's dusk.
            (Place::Dream, _) => Air {
                count: 30,
                color: [1.1, 0.75, 1.6],
                opacity: 0.75,
                size: 0.016,
                flow: Vec3::new(0.0, 0.02, 0.0),
                wander: 0.05,
                twinkle: 1.2,
            },
            (Place::Ruins, _) => Air {
                count: 40,
                color: [1.2, 1.15, 1.0],
                opacity: 0.8,
                size: 0.018,
                flow: Vec3::new(0.06, 0.01, 0.03),
                wander: 0.07,
                twinkle: 2.0,
            },
            (_, Some(Theme::Tombs)) => Air {
                count: 48,
                color: [1.3, 1.0, 0.45],
                twinkle: 3.0,
                ..dust
            },
            (_, Some(Theme::Catacombs)) => Air {
                color: [0.85, 0.82, 0.76],
                flow: Vec3::new(0.0, -0.04, 0.0),
                ..dust
            },
            (_, Some(Theme::FallenKeep)) => Air {
                count: 30,
                color: [0.6, 0.6, 0.66],
                opacity: 0.8,
                size: 0.02,
                flow: Vec3::new(0.02, -0.07, 0.0),
                wander: 0.04,
                twinkle: 0.0,
            },
            (_, Some(Theme::EmberDeep)) => Air {
                count: 44,
                color: [2.6, 1.0, 0.3],
                opacity: 1.0,
                size: 0.028,
                flow: Vec3::new(0.0, 0.22, 0.0),
                wander: 0.12,
                twinkle: 6.0,
            },
            (_, Some(Theme::Haunted)) => Air {
                count: 12,
                color: [0.6, 0.9, 1.1],
                opacity: 0.22,
                size: 0.09,
                flow: Vec3::new(0.05, 0.02, -0.03),
                wander: 0.08,
                twinkle: 0.6,
            },
            _ => dust,
        }
    }
}

#[derive(Resource)]
struct MoteAssets {
    mesh: Handle<Mesh>,
    dot: Handle<Image>,
}

#[derive(Component)]
struct Mote {
    /// This mote's own wandering, and its phase in the twinkle.
    wander: Vec3,
    phase: f32,
    size: f32,
}

fn load(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
) {
    // A soft round dot: bright centre, fading edge.
    let mut data = Vec::with_capacity((DOT * DOT * 4) as usize);
    for y in 0..DOT {
        for x in 0..DOT {
            let d = Vec2::new(x as f32 + 0.5, y as f32 + 0.5)
                .distance(Vec2::splat(DOT as f32 / 2.0))
                / (DOT as f32 / 2.0);
            let a = (1.0 - d).clamp(0.0, 1.0).powf(1.5);
            data.extend_from_slice(&[255, 255, 255, (a * 255.0) as u8]);
        }
    }
    let dot = Image::new(
        Extent3d {
            width: DOT,
            height: DOT,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    commands.insert_resource(MoteAssets {
        mesh: meshes.add(Rectangle::new(1.0, 1.0)),
        dot: images.add(dot),
    });
}

/// Fill the air for the place you're in; a new place gets new air.
#[allow(clippy::too_many_arguments)]
fn refill(
    mut commands: Commands,
    run: Res<Run>,
    assets: Res<MoteAssets>,
    cams: Query<&Transform, With<PlayerCamera>>,
    motes: Query<Entity, With<Mote>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut current: Local<Option<(Air, Handle<StandardMaterial>)>>,
) {
    let Ok(cam) = cams.single() else {
        return;
    };
    let air = Air::of(run.floor.place, run.floor.stage.map(|s| s.theme));
    if current.as_ref().is_some_and(|(a, _)| *a == air) {
        return;
    }
    for e in &motes {
        commands.entity(e).despawn();
    }
    if let Some((_, old)) = current.take() {
        materials.remove(&old);
    }
    let [r, g, b] = air.color;
    let material = materials.add(StandardMaterial {
        base_color: Color::LinearRgba(LinearRgba::new(r, g, b, air.opacity)),
        base_color_texture: Some(assets.dot.clone()),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        cull_mode: None,
        ..default()
    });
    let centre = centre(cam);
    for i in 0..air.count as u64 {
        let h = |k: u64| (zrpg_core::rng::mix64(i * 7 + k) % 10_000) as f32 / 10_000.0;
        let at = centre + (Vec3::new(h(1), h(2), h(3)) * 2.0 - 1.0) * HALF_BOX;
        let wander = (Vec3::new(h(4), h(5), h(6)) * 2.0 - 1.0) * air.wander;
        let size = air.size * (0.6 + 0.8 * h(7));
        commands.spawn((
            Mote {
                wander,
                phase: h(8) * std::f32::consts::TAU,
                size,
            },
            Mesh3d(assets.mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(at).with_scale(Vec3::splat(size)),
            NotShadowCaster,
        ));
    }
    *current = Some((air, material));
}

fn centre(cam: &Transform) -> Vec3 {
    cam.translation + cam.forward().with_y(0.0).normalize_or_zero() * AHEAD
}

/// Drift, wander, twinkle, face the camera, and wrap around the box.
fn drift(
    time: Res<Time>,
    run: Res<Run>,
    cams: Query<&Transform, (With<PlayerCamera>, Without<Mote>)>,
    mut motes: Query<(&Mote, &mut Transform)>,
) {
    let Ok(cam) = cams.single() else {
        return;
    };
    let air = Air::of(run.floor.place, run.floor.stage.map(|s| s.theme));
    let (dt, t) = (time.delta_secs(), time.elapsed_secs());
    let centre = centre(cam);
    for (m, mut tf) in &mut motes {
        // Wander in slow curls around the steady flow.
        let curl = Vec3::new(
            (t * 0.7 + m.phase).sin(),
            (t * 0.9 + m.phase * 1.3).cos(),
            (t * 0.6 + m.phase * 0.7).cos(),
        );
        let mut p = tf.translation + (air.flow + m.wander + curl * air.wander * 0.5) * dt;
        let rel = p - centre;
        let wrapped = Vec3::new(
            wrap(rel.x, HALF_BOX.x),
            wrap(rel.y, HALF_BOX.y),
            wrap(rel.z, HALF_BOX.z),
        );
        if wrapped != rel {
            p = centre + wrapped;
        }
        let glint = if air.twinkle > 0.0 {
            0.55 + 0.45 * (t * air.twinkle + m.phase).sin()
        } else {
            1.0
        };
        tf.translation = p;
        tf.rotation = cam.rotation;
        tf.scale = Vec3::splat(m.size * glint);
    }
}

/// Bring `x` back into `-half..half`, coming in from the other side.
fn wrap(x: f32, half: f32) -> f32 {
    (x + half).rem_euclid(2.0 * half) - half
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motes_wrap_to_the_far_side() {
        assert_eq!(wrap(0.5, 1.0), 0.5);
        assert!((wrap(1.2, 1.0) - -0.8).abs() < 1e-5);
        assert!((wrap(-1.3, 1.0) - 0.7).abs() < 1e-5);
    }

    #[test]
    fn every_place_has_its_own_air() {
        let mut seen = Vec::new();
        for theme in Theme::ALL {
            let air = Air::of(Place::Dungeon, Some(theme));
            assert!(air.count > 0);
            seen.push(air);
        }
        seen.push(Air::of(Place::Ruins, None));
        // The crypt keeps the plain dust; the others differ from it.
        let distinct = seen
            .iter()
            .filter(|a| **a != Air::of(Place::Dungeon, Some(Theme::Crypt)))
            .count();
        assert_eq!(distinct, seen.len() - 1);
    }
}
