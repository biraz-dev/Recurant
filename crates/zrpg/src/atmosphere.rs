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
use zrpg_core::{Floor, Place, Theme};

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
    /// The colour of torchlight and flames.
    pub torch: Color,
}

impl Look {
    pub const DUNGEON: Look = Look {
        sky: FOG_COLOR,
        fog: FOG,
        grade: DepthGradeUniform {
            params: Vec4::new(GRADE_NEAR, GRADE_FAR, DESATURATE, DARKEN),
            tint: Vec4::new(DRAIN_TINT.x, DRAIN_TINT.y, DRAIN_TINT.z, 1.0),
            dream: Vec4::ZERO,
        },
        ambient_color: crate::lighting::AMBIENT_COLOR,
        ambient_brightness: crate::lighting::AMBIENT_BRIGHTNESS,
        view_distance: 9.0,
        eye_height: crate::view::EYE_HEIGHT,
        torch: crate::lighting::TORCH_COLOR,
    };

    /// Sunlit sandstone: the far court fades into a warm, bright haze, and
    /// shadows are a muted cool green, as in the kit's reference.
    pub const RUINS: Look = Look {
        sky: Color::srgb(0.84, 0.83, 0.78),
        fog: FogFalloff::ExponentialSquared { density: 0.055 },
        grade: DepthGradeUniform {
            params: Vec4::new(5.0, 22.0, 0.45, 0.9),
            tint: Vec4::new(1.06, 1.0, 0.86, 1.0),
            dream: Vec4::ZERO,
        },
        ambient_color: Color::srgb(0.7, 0.82, 0.76),
        ambient_brightness: 70.0,
        view_distance: 32.0,
        // The kit's proposed 1.5 m. It also keeps the eye well clear of the
        // walls' block-course ledges (at 1 m and 2 m), which would otherwise
        // show as a bright line on the horizon whenever a hit shakes you.
        eye_height: 1.5 * crate::ruins::SCALE,
        torch: crate::lighting::TORCH_COLOR,
    };

    /// The Mindglass Sanctum: violet dusk indoors. Its crystals give most
    /// of the light; the haze deepens to violet-black, colours drain toward
    /// amethyst, and the eye is at the kit's tall-profile height (its
    /// doorways need 2.4 m; a dungeon's 2 m would not do).
    pub const SANCTUM: Look = Look {
        sky: Color::srgb(0.045, 0.03, 0.075),
        fog: FogFalloff::ExponentialSquared { density: 0.085 },
        grade: DepthGradeUniform {
            params: Vec4::new(3.0, 13.0, 0.5, 0.55),
            tint: Vec4::new(0.95, 0.82, 1.18, 1.0),
            dream: Vec4::ZERO,
        },
        ambient_color: Color::srgb(0.72, 0.62, 0.98),
        ambient_brightness: 60.0,
        view_distance: 18.0,
        eye_height: 1.5 * crate::ruins::SCALE,
        // The hand torch burns a little cooler here.
        torch: Color::srgb(1.0, 0.86, 0.92),
    };

    /// How a floor looks: the ruins, the sanctum, or a dungeon stage's own
    /// take on the dark.
    pub fn of(floor: &Floor) -> Look {
        match (floor.place, floor.stage) {
            (Place::Ruins, _) => Look::RUINS,
            // Patchworks start from the sanctum's look; their rooms' own air
            // is blended in as you walk (`dreamscape::thresholds`).
            (Place::Dream(_), _) => Look::SANCTUM,
            (Place::Dungeon, Some(stage)) => Look::stage(stage.theme),
            (Place::Dungeon, None) => Look::DUNGEON,
        }
    }

    /// Each stage tints the dungeon's haze, shadows and torchlight.
    fn stage(theme: Theme) -> Look {
        let d = Look::DUNGEON;
        let tint = |sky: Color, ambient: Color, torch: Color, drain: Vec3| Look {
            sky,
            ambient_color: ambient,
            torch,
            grade: DepthGradeUniform {
                tint: drain.extend(1.0),
                ..d.grade
            },
            ..d.clone()
        };
        match theme {
            // The original dungeon.
            Theme::Crypt => d,
            // Bone-dust: a dry, warm grey.
            Theme::Catacombs => tint(
                Color::srgb(0.055, 0.05, 0.045),
                Color::srgb(0.82, 0.78, 0.72),
                Color::srgb(1.0, 0.86, 0.66),
                Vec3::new(0.95, 0.92, 0.88),
            ),
            // Gilded dust and bright flames.
            Theme::Tombs => tint(
                Color::srgb(0.065, 0.052, 0.032),
                Color::srgb(0.9, 0.8, 0.6),
                Color::srgb(1.0, 0.8, 0.5),
                Vec3::new(1.05, 0.95, 0.75),
            ),
            // Red haze and hot, low fires.
            Theme::EmberDeep => tint(
                Color::srgb(0.08, 0.03, 0.02),
                Color::srgb(0.95, 0.55, 0.42),
                Color::srgb(1.0, 0.55, 0.3),
                Vec3::new(1.1, 0.8, 0.7),
            ),
            // Cold blue, and ghostly torches.
            Theme::Haunted => Look {
                fog: FogFalloff::ExponentialSquared { density: 0.3 },
                ..tint(
                    Color::srgb(0.03, 0.05, 0.07),
                    Color::srgb(0.6, 0.75, 0.95),
                    Color::srgb(0.6, 0.95, 1.0),
                    Vec3::new(0.75, 0.9, 1.1),
                )
            },
            // Steel-grey stone, plain firelight.
            Theme::FallenKeep => tint(
                Color::srgb(0.042, 0.045, 0.055),
                Color::srgb(0.72, 0.75, 0.84),
                Color::srgb(1.0, 0.82, 0.62),
                Vec3::new(0.85, 0.9, 1.0),
            ),
        }
    }

    /// The haze colour that, once tonemapped, matches the sky (which is not
    /// tonemapped), so the far distance melts into it without a seam.
    pub fn fog_color(&self) -> Color {
        untonemapped(self.sky)
    }

    /// Set the colour distance drains toward.
    pub fn set_tint(&mut self, tint: Vec3) {
        self.grade.tint = tint.extend(1.0);
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
pub(crate) fn apply_look(
    run: Res<Run>,
    mut current: Local<Option<(Place, Option<Theme>)>>,
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
    let place = (run.floor.place, run.floor.stage.map(|s| s.theme));
    if *current == Some(place) {
        return;
    }
    let look = Look::of(&run.floor);
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
    /// Dissolving like a dream: x is how far gone (0 whole, 1 vanished), y
    /// the size of the pixel grid it breaks up on. A blow landing: z its
    /// force, already fading (0 none, 1 the hardest), w seconds since.
    dream: Vec4,
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct DepthGrade {
    #[uniform(100)]
    grade: DepthGradeUniform,
}

impl DepthGrade {
    /// Dissolve the surface: `amount` 0 leaves it whole, 1 takes it all;
    /// it breaks up on a grid of `pixels` across its texture.
    pub fn set_dissolve(&mut self, amount: f32, pixels: f32) {
        self.grade.dream.x = amount;
        self.grade.dream.y = pixels;
    }

    pub fn dissolve(&self) -> f32 {
        self.grade.dream.x
    }

    /// Ripple the surface from a blow of `force` (0..1, fading) that landed
    /// `age` seconds ago; force 0 leaves it still.
    pub fn set_impact(&mut self, force: f32, age: f32) {
        self.grade.dream.z = force;
        self.grade.dream.w = age;
    }

    pub fn impact(&self) -> (f32, f32) {
        (self.grade.dream.z, self.grade.dream.w)
    }
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
