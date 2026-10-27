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

use bevy::asset::embedded_asset;
use bevy::pbr::{ExtendedMaterial, FogFalloff, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

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

/// Wrap a standard material in the shared depth grading.
pub fn graded(base: StandardMaterial) -> GradedMaterial {
    GradedMaterial {
        base,
        extension: DepthGrade::default(),
    }
}

pub struct AtmospherePlugin;

impl Plugin for AtmospherePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "depth_grade.wgsl");
        app.add_plugins(MaterialPlugin::<GradedMaterial>::default());
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

impl Default for DepthGrade {
    fn default() -> Self {
        Self {
            grade: DepthGradeUniform {
                params: Vec4::new(GRADE_NEAR, GRADE_FAR, DESATURATE, DARKEN),
                tint: DRAIN_TINT.extend(1.0),
            },
        }
    }
}

impl MaterialExtension for DepthGrade {
    fn fragment_shader() -> ShaderRef {
        "embedded://zrpg/depth_grade.wgsl".into()
    }
}
