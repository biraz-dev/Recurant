//! Atmosphere: what distance does to the picture. Three layers, all tunable
//! here:
//!
//! - **desaturation**: colours drain toward a cool grey with distance;
//! - **darkness**: brightness falls away with distance, beyond what the
//!   torchlight's own falloff does;
//! - **fog**: a dusty haze that thickens with distance (Bevy's distance fog,
//!   exponential-squared so the near view stays clear).
//!
//! Desaturation and darkness live in a small shader (`depth_grade.wgsl`) that
//! extends Bevy's standard material: lighting as usual, then grading, then
//! fog and tonemapping. Torch flames skip all of it, so they burn through.
//!
//! Each [`Place`] has its own [`Look`]: the dungeon is dark and close; the
//! sunlit ruins have a warm, far haze and a sky.

use bevy::asset::embedded_asset;
use bevy::pbr::{ExtendedMaterial, FogFalloff, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use zrpg_core::Place;

use crate::view::PlayerCamera;
use crate::{Frame, Run};

/// Grading starts at this distance (world units; a cell is 1.0)...
const GRADE_NEAR: f32 = 1.2;
/// ...and is complete here.
const GRADE_FAR: f32 = 6.0;
/// How much colour is gone at full distance (0 = none, 1 = grey).
const DESATURATE: f32 = 0.85;
/// Brightness left at full distance.
const DARKEN: f32 = 0.35;
/// Far colours drain toward a cool grey, not a neutral one.
const DRAIN_TINT: Vec3 = Vec3::new(0.82, 0.9, 1.05);

/// The haze. Also the background colour, so open space reads as deep haze.
pub const FOG_COLOR: Color = Color::srgb(0.045, 0.047, 0.058);
/// Exponential-squared: clear up close, thick at the far end of a corridor.
pub const FOG: FogFalloff = FogFalloff::ExponentialSquared { density: 0.26 };

/// A standard PBR material with depth grading on top.
pub type GradedMaterial = ExtendedMaterial<StandardMaterial, DepthGrade>;

/// Wrap a standard material in the dungeon's depth grading.
pub fn graded(base: StandardMaterial) -> GradedMaterial {
    Look::DUNGEON.graded(base)
}

/// Everything about how a place looks at a distance, and how it is lit
/// when no torch is near.
#[derive(Clone, Debug)]
pub struct Look {
    /// Background: open space and the sky. Distant surfaces fade into it.
    pub sky: Color,
    pub fog: FogFalloff,
    grade: DepthGradeUniform,
    pub ambient_color: Color,
    pub ambient_brightness: f32,
    /// Far clipping distance, in world units.
    pub view_distance: f32,
    /// How high the player's eyes are above the ground.
    pub eye_height: f32,
}

impl Look {
    pub const DUNGEON: Look = Look {
        sky: FOG_COLOR,
        fog: FOG,
        grade: DepthGradeUniform {
            params: Vec4::new(GRADE_NEAR, GRADE_FAR, DESATURATE, DARKEN),
            tint: Vec4::new(DRAIN_TINT.x, DRAIN_TINT.y, DRAIN_TINT.z, 1.0),
        },
        ambient_color: crate::lighting::AMBIENT_COLOR,
        ambient_brightness: crate::lighting::AMBIENT_BRIGHTNESS,
        view_distance: 9.0,
        eye_height: crate::view::EYE_HEIGHT,
    };

    /// Sunlit sandstone: the far court fades into a warm, bright haze, and
    /// shadows are a muted cool green, as in the kit's reference.
    pub const RUINS: Look = Look {
        sky: Color::srgb(0.84, 0.83, 0.78),
        fog: FogFalloff::ExponentialSquared { density: 0.055 },
        grade: DepthGradeUniform {
            params: Vec4::new(5.0, 22.0, 0.45, 0.9),
            tint: Vec4::new(1.06, 1.0, 0.86, 1.0),
        },
        ambient_color: Color::srgb(0.7, 0.82, 0.76),
        ambient_brightness: 70.0,
        view_distance: 32.0,
        // The kit's proposed 1.5 m. It also keeps the eye well clear of the
        // walls' block-course ledges (at 1 m and 2 m), which would otherwise
        // show as a bright line on the horizon whenever a hit shakes you.
        eye_height: 1.5 * crate::ruins::SCALE,
    };

    pub fn of(place: Place) -> Look {
        match place {
            Place::Dungeon => Look::DUNGEON,
            Place::Ruins => Look::RUINS,
        }
    }

    /// The haze colour that, once tonemapped, matches the sky (which is not
    /// tonemapped), so the far distance melts into it without a seam.
    pub fn fog_color(&self) -> Color {
        untonemapped(self.sky)
    }

    /// Wrap a standard material in this place's depth grading.
    pub fn graded(&self, base: StandardMaterial) -> GradedMaterial {
        GradedMaterial {
            base,
            extension: DepthGrade { grade: self.grade },
        }
    }
}

/// The colour that, once tonemapped, shows as `color`: for things drawn
/// to match the untonemapped background.
pub fn untonemapped(color: Color) -> Color {
    let c = color.to_linear();
    let luma = c.luminance().min(0.95);
    // Inverse of Reinhard on luminance: out = in / (1 + in).
    let k = 1.0 / (1.0 - luma);
    Color::linear_rgb(c.red * k, c.green * k, c.blue * k)
}

pub struct AtmospherePlugin;

impl Plugin for AtmospherePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "depth_grade.wgsl");
        app.add_plugins(MaterialPlugin::<GradedMaterial>::default())
            .add_systems(Update, apply_look.in_set(Frame::Present));
    }
}

/// Dress the camera for the place the player is in, whenever that changes.
fn apply_look(
    run: Res<Run>,
    mut current: Local<Option<Place>>,
    mut cams: Query<
        (
            &mut Camera,
            &mut DistanceFog,
            &mut AmbientLight,
            &mut Projection,
        ),
        With<PlayerCamera>,
    >,
) {
    let place = run.floor.place;
    if *current == Some(place) {
        return;
    }
    let look = Look::of(place);
    for (mut camera, mut fog, mut ambient, mut projection) in &mut cams {
        camera.clear_color = ClearColorConfig::Custom(look.sky);
        fog.color = look.fog_color();
        fog.falloff = look.fog.clone();
        ambient.color = look.ambient_color;
        ambient.brightness = look.ambient_brightness;
        if let Projection::Perspective(p) = projection.as_mut() {
            p.far = look.view_distance;
        }
        *current = Some(place);
    }
}

#[derive(Clone, Copy, Debug, Reflect, ShaderType)]
pub struct DepthGradeUniform {
    /// near, far, desaturation, brightness at far.
    params: Vec4,
    /// rgb: tint of the drained colour.
    tint: Vec4,
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct DepthGrade {
    #[uniform(100)]
    grade: DepthGradeUniform,
}

impl MaterialExtension for DepthGrade {
    fn fragment_shader() -> ShaderRef {
        "embedded://zrpg/depth_grade.wgsl".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reinhard on luminance, as the camera's tonemapping does it.
    fn tonemap(c: LinearRgba) -> LinearRgba {
        let l = c.luminance();
        let k = (l / (1.0 + l)) / l;
        LinearRgba::rgb(c.red * k, c.green * k, c.blue * k)
    }

    #[test]
    fn haze_meets_the_sky() {
        for look in [Look::DUNGEON, Look::RUINS] {
            let sky = look.sky.to_linear();
            let fog = tonemap(look.fog_color().to_linear());
            for (a, b) in [
                (sky.red, fog.red),
                (sky.green, fog.green),
                (sky.blue, fog.blue),
            ] {
                assert!((a - b).abs() < 1e-4, "{:?}: {sky:?} vs {fog:?}", look.sky);
            }
        }
    }
}
