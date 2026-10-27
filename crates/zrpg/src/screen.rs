//! Low-resolution rendering: the 3D view draws into a 320x240 image, which
//! a 2D camera then scales up (by whole pixels when possible) to the window.

use bevy::camera::RenderTarget;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::window::PrimaryWindow;

pub const WIDTH: u32 = 320;
pub const HEIGHT: u32 = 240;

/// The offscreen image the game world renders into.
#[derive(Resource)]
pub struct LowResTarget(pub Handle<Image>);

#[derive(Component)]
struct Upscaled;

pub struct ScreenPlugin;

impl Plugin for ScreenPlugin {
    fn build(&self, app: &mut App) {
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        let image = Image::new_target_texture(WIDTH, HEIGHT, TextureFormat::Rgba8UnormSrgb, None);
        let handle = images.add(image);
        app.insert_resource(LowResTarget(handle))
            .add_systems(Startup, spawn_presenter)
            .add_systems(PostUpdate, fit_to_window);
    }
}

fn spawn_presenter(mut commands: Commands, target: Res<LowResTarget>) {
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            ..default()
        },
        Tonemapping::None,
        Msaa::Off,
    ));
    commands.spawn((Sprite::from_image(target.0.clone()), Upscaled));
}

/// Render-target settings for the world camera.
pub fn world_target(target: &LowResTarget) -> RenderTarget {
    target.0.clone().into()
}

fn fit_to_window(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut sprites: Query<&mut Transform, With<Upscaled>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let (pw, ph) = (
        window.physical_width() as f32,
        window.physical_height() as f32,
    );
    let fit = (pw / WIDTH as f32).min(ph / HEIGHT as f32);
    // Whole-pixel scaling keeps pixels crisp; fall back to fractional below 1x.
    let physical_scale = if fit >= 1.0 { fit.floor() } else { fit };
    let scale = physical_scale / window.scale_factor();
    for mut t in &mut sprites {
        t.scale = Vec3::new(scale, scale, 1.0);
    }
}
