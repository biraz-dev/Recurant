//! Procedural placeholder textures, so the game needs no asset files yet.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use zrpg_core::rng::mix64;

pub const SIZE: u32 = 32;

/// Deterministic per-pixel noise in `-1.0..1.0`.
fn noise(salt: u64, x: u32, y: u32) -> f32 {
    let h = mix64(salt ^ ((x as u64) << 32 | y as u64));
    (h & 0xFFFF) as f32 / 32767.5 - 1.0
}

fn build(salt: u64, pixel: impl Fn(u32, u32, f32) -> [u8; 3]) -> Image {
    let mut data = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let [r, g, b] = pixel(x, y, noise(salt, x, y));
            data.extend_from_slice(&[r, g, b, 255]);
        }
    }
    Image::new(
        Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

fn shade(base: [u8; 3], k: f32) -> [u8; 3] {
    base.map(|c| (c as f32 * k).clamp(0.0, 255.0) as u8)
}

/// Rows of offset bricks.
pub fn wall() -> Image {
    build(1, |x, y, n| {
        let row = y / 8;
        let bx = (x + if row % 2 == 1 { 8 } else { 0 }) % 16;
        if y % 8 == 7 || bx == 15 {
            return shade([40, 34, 30], 1.0 + n * 0.15);
        }
        let brick = noise(2, (x + row * 8) / 16, row);
        let edge = if y % 8 == 0 || bx == 0 { 1.15 } else { 1.0 };
        shade([112, 92, 76], (0.85 + brick * 0.12 + n * 0.06) * edge)
    })
}

/// Large flagstones.
pub fn floor() -> Image {
    build(3, |x, y, n| {
        if x % 16 == 0 || y % 16 == 0 {
            return shade([30, 30, 34], 1.0 + n * 0.2);
        }
        let stone = noise(4, x / 16, y / 16);
        shade([78, 76, 82], 0.9 + stone * 0.1 + n * 0.08)
    })
}

pub fn ceiling() -> Image {
    build(5, |x, y, n| {
        let beam = x % 32 < 2;
        let base = if beam { [44, 34, 26] } else { [52, 50, 56] };
        shade(base, 0.85 + n * 0.1 + if y % 4 == 0 { -0.05 } else { 0.0 })
    })
}

/// Steps descending into darkness, drawn on the floor.
pub fn stairs() -> Image {
    build(6, |_x, y, n| {
        let step = y / 5; // 0 = far end (darkest)
        if step >= 6 {
            return shade([78, 76, 82], 0.9 + n * 0.08);
        }
        let lit = step as f32 / 6.0;
        let lip = if y % 5 == 4 { 1.3 } else { 1.0 };
        shade([96, 90, 84], (0.15 + lit * 0.7) * lip + n * 0.04)
    })
}
