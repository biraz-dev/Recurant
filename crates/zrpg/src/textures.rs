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
    build_rgba(SIZE, SIZE, salt, |x, y, n| {
        let [r, g, b] = pixel(x, y, n);
        Some([r, g, b])
    })
}

/// `None` pixels are fully transparent (for alpha-masked sprites).
fn build_rgba(
    w: u32,
    h: u32,
    salt: u64,
    pixel: impl Fn(u32, u32, f32) -> Option<[u8; 3]>,
) -> Image {
    let mut data = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            match pixel(x, y, noise(salt, x, y)) {
                Some([r, g, b]) => data.extend_from_slice(&[r, g, b, 255]),
                None => data.extend_from_slice(&[0, 0, 0, 0]),
            }
        }
    }
    Image::new(
        Extent3d {
            width: w,
            height: h,
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

/// Dressed stone for steps: four 8-pixel courses, each with a worn bright
/// nosing and a shadowed back edge.
pub fn stairs() -> Image {
    build(6, |x, y, n| {
        let course = y / 8;
        let block = noise(9, (x + course * 8) / 16, course);
        let k = match y % 8 {
            7 => 1.35,
            0 => 0.6,
            _ => 1.0,
        };
        let seam = if (x + course * 8) % 16 == 0 { 0.7 } else { 1.0 };
        shade([104, 98, 92], (0.9 + block * 0.08 + n * 0.06) * k * seam)
    })
}

/// The wall the stairs lead under: bricks around a dark arched passage.
pub fn arch() -> Image {
    let bricks = wall();
    let data = bricks.data.as_deref().unwrap_or_default();
    build(10, |x, y, n| {
        // Opening: 20px wide, straight sides up to y=14, round top above.
        let (fx, fy) = (x as f32 + 0.5 - 16.0, y as f32 + 0.5);
        let r = (fx * fx + (fy - 14.0).powi(2)).sqrt();
        let inside = fx.abs() < 10.0 && (fy > 14.0 || r < 10.0);
        let ring = !inside && fx.abs() < 13.0 && (fy > 14.0 || r < 13.0);
        if inside {
            // Darker towards the middle and bottom: the passage drops away.
            let depth = (1.0 - fx.abs() / 10.0) * 0.6 + fy / 32.0 * 0.4;
            return shade([40, 34, 30], (1.0 - depth) * 0.5 + n * 0.03);
        }
        if ring {
            let k = if r > 12.0 || fx.abs() > 12.0 {
                0.6
            } else {
                1.0
            };
            return shade([138, 120, 100], k + n * 0.06);
        }
        let i = ((y * SIZE + x) * 4) as usize;
        [data[i], data[i + 1], data[i + 2]]
    })
}

/// A slime: a squat dome with a highlight, two eyes and a mouth.
pub fn slime() -> Image {
    build_rgba(SIZE, SIZE, 7, |x, y, n| {
        let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
        // Dome: half-ellipse sitting on the bottom edge.
        let (dx, dy) = ((fx - 16.0) / 14.0, (fy - 31.0) / 22.0);
        let r = dx * dx + dy * dy;
        if r > 1.0 || fy > 31.0 {
            return None;
        }
        let eye = |cx: f32| {
            let d = (fx - cx).powi(2) + (fy - 18.0).powi(2);
            if d < 2.0 {
                Some([10, 14, 10])
            } else if d < 7.0 {
                Some([236, 240, 220])
            } else {
                None
            }
        };
        if let Some(c) = eye(11.0).or_else(|| eye(21.0)) {
            return Some(c);
        }
        if (fy - 24.5).abs() < 1.0 && (fx - 16.0).abs() < 4.0 {
            return Some([20, 40, 18]);
        }
        let highlight = (fx - 10.0).powi(2) + (fy - 13.0).powi(2) < 6.0;
        let rim = r > 0.82;
        let k = if highlight {
            1.5
        } else if rim {
            0.6
        } else {
            1.0 + n * 0.06
        };
        Some(shade([84, 168, 72], k * (0.8 + (1.0 - fy / 32.0) * 0.4)))
    })
}

/// The player's sword, held point-up, for the first-person hand overlay.
pub const SWORD_SIZE: (u32, u32) = (16, 48);

pub fn sword() -> Image {
    let (w, h) = SWORD_SIZE;
    build_rgba(w, h, 8, |x, y, n| {
        let cx = (x as i32 - 7).abs();
        match y {
            // Point.
            0..=3 if cx as u32 <= y / 2 => Some(shade([210, 214, 224], 1.0 + n * 0.05)),
            // Blade with a darker fuller down the middle.
            4..=31 if cx <= 2 => {
                let k = if cx == 0 {
                    0.75
                } else if x < 7 {
                    1.1
                } else {
                    0.9
                };
                Some(shade([196, 200, 212], k + n * 0.05))
            }
            // Crossguard.
            32..=34 if cx <= 7 => Some(shade([150, 112, 40], 1.0 - (y - 32) as f32 * 0.15)),
            // Grip.
            35..=44 if cx <= 1 => Some(shade([84, 50, 30], if y % 2 == 0 { 1.0 } else { 0.7 })),
            // Pommel.
            45..=47 if cx <= 2 => Some(shade([170, 130, 50], 1.0)),
            _ => None,
        }
    })
}
