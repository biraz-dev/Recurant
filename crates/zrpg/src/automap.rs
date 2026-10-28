//! The automap fills in as you explore. It shows only what you have seen
//! (cells within a few steps with a clear line of sight), so the unexplored
//! stays dark. Creatures are not on it.
//!
//! Tab (gamepad: LT) cycles it: a small, faint map in the top-left corner,
//! then the whole map over a dimmed screen, then hidden.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use zrpg_core::{Facing, GridPos, Map, Tile};

use crate::input::{Action, Actions};
use crate::view::PlayerCamera;
use crate::{Frame, Run};

/// Map pixels per cell (the full-screen map scales this up).
const CELL: u32 = 3;
/// How far you can see to map, in cells.
const SIGHT: i32 = 4;
/// How opaque the corner map and the full-screen map are.
const CORNER_OPACITY: f32 = 0.75;
const FULL_OPACITY: f32 = 0.95;
/// The full-screen map dims the world behind it.
const FULL_BACKDROP: Color = Color::srgba(0.0, 0.0, 0.0, 0.78);
/// Room the full-screen map may fill, in screen pixels.
const FULL_ROOM: (u32, u32) = (300, 210);
/// Dark floors and pale walls read over both the bright ruins and the
/// dark dungeon.
const FLOOR: [u8; 4] = [18, 14, 10, 150];
const WALL: [u8; 4] = [235, 215, 170, 200];
const STAIRS: [u8; 4] = [150, 205, 255, 230];
const YOU: [u8; 4] = [242, 191, 74, 255];

pub struct AutomapPlugin;

impl Plugin for AutomapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Explored>()
            .init_resource::<MapMode>()
            .add_systems(PostStartup, spawn)
            .add_systems(
                Update,
                (cycle, explore, draw).chain().in_set(Frame::Present),
            );
    }
}

/// How the map is shown. Starts hidden; Tab steps through the rest.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MapMode {
    #[default]
    Hidden,
    Corner,
    Full,
}

impl MapMode {
    fn next(self) -> MapMode {
        match self {
            MapMode::Hidden => MapMode::Corner,
            MapMode::Corner => MapMode::Full,
            MapMode::Full => MapMode::Hidden,
        }
    }
}

/// Cells seen on the current floor.
#[derive(Resource, Default)]
struct Explored {
    floor: Option<(u64, u32)>,
    seen: Vec<bool>,
    /// What the map was last drawn for, to redraw only on change.
    drawn: Option<(GridPos, Facing, usize)>,
}

/// The layer the map sits in: clear around the corner map, dimmed behind
/// the full one.
#[derive(Component)]
struct AutomapLayer;
#[derive(Component)]
struct AutomapImage;

fn spawn(
    mut commands: Commands,
    cams: Query<Entity, With<PlayerCamera>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Ok(cam) = cams.single() else {
        return;
    };
    let image = images.add(blank(1, 1));
    commands
        .spawn((
            AutomapLayer,
            UiTargetCamera(cam),
            // Over the HUD, under the ability picker.
            GlobalZIndex(5),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                ..default()
            },
            BackgroundColor(Color::NONE),
            Visibility::Hidden,
        ))
        .with_children(|layer| {
            layer.spawn((
                AutomapImage,
                ImageNode::new(image),
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
            ));
        });
}

fn blank(w: u32, h: u32) -> Image {
    Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![0; (w * h * 4) as usize],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
}

fn cycle(actions: Res<Actions>, mut mode: ResMut<MapMode>) {
    if actions.just_pressed(Action::Map) {
        *mode = mode.next();
    }
}

/// Can you see from `a` to `b`? Every cell strictly between them must be
/// open (walls hide what is behind them, not themselves).
fn in_sight(map: &Map, a: GridPos, b: GridPos) -> bool {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let steps = dx.abs().max(dy.abs());
    (1..steps).all(|i| {
        let t = i as f32 / steps as f32;
        let p = GridPos::new(
            (a.x as f32 + dx as f32 * t).round() as i32,
            (a.y as f32 + dy as f32 * t).round() as i32,
        );
        map.is_walkable(p)
    })
}

fn explore(run: Res<Run>, mut explored: ResMut<Explored>) {
    let map = &run.floor.map;
    let floor = Some((run.run_seed, run.depth));
    if explored.floor != floor {
        explored.floor = floor;
        explored.seen = vec![false; (map.width() * map.height()) as usize];
        explored.drawn = None;
    }
    let me = run.player.pos;
    for y in me.y - SIGHT..=me.y + SIGHT {
        for x in me.x - SIGHT..=me.x + SIGHT {
            let p = GridPos::new(x, y);
            if !map.in_bounds(p) || me.manhattan(p) > SIGHT + 1 {
                continue;
            }
            let i = map.index(p);
            if !explored.seen[i] && in_sight(map, me, p) {
                explored.seen[i] = true;
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn draw(
    run: Res<Run>,
    mode: Res<MapMode>,
    picker: Res<crate::picker::Picker>,
    mut explored: ResMut<Explored>,
    mut images: ResMut<Assets<Image>>,
    mut layers: Query<(&mut Visibility, &mut BackgroundColor), With<AutomapLayer>>,
    mut nodes: Query<(&mut ImageNode, &mut Node), With<AutomapImage>>,
) {
    let (Ok((mut vis, mut backdrop)), Ok((mut image, mut node))) =
        (layers.single_mut(), nodes.single_mut())
    else {
        return;
    };
    let show = *mode != MapMode::Hidden && run.player.is_alive() && !picker.is_open();
    let want = if show {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if *vis != want {
        *vis = want;
    }
    if !show {
        return;
    }

    // Place it: small in the corner, or as large as fits in the middle of
    // the screen (in whole pixels per map pixel, so cells stay crisp).
    let map = &run.floor.map;
    let (w, h) = (map.width() as u32 * CELL, map.height() as u32 * CELL);
    let (scale, left, top, opacity, back) = if *mode == MapMode::Full {
        let k = (FULL_ROOM.0 / w).min(FULL_ROOM.1 / h).max(1);
        let left = crate::screen::WIDTH.saturating_sub(w * k) / 2;
        let top = crate::screen::HEIGHT.saturating_sub(h * k) / 2;
        (k, left, top, FULL_OPACITY, FULL_BACKDROP)
    } else {
        (1, 5, 5, CORNER_OPACITY, Color::NONE)
    };
    if backdrop.0 != back {
        backdrop.0 = back;
    }
    let tint = Color::srgba(1.0, 1.0, 1.0, opacity);
    if image.color != tint {
        image.color = tint;
    }
    let placed = (
        px((w * scale) as f32),
        px((h * scale) as f32),
        px(left as f32),
        px(top as f32),
    );
    if (node.width, node.height, node.left, node.top) != placed {
        (node.width, node.height, node.left, node.top) = placed;
    }

    let p = &run.player;
    let seen_count = explored.seen.iter().filter(|s| **s).count();
    let state = Some((p.pos, p.facing, seen_count));
    if explored.drawn == state {
        return;
    }
    explored.drawn = state;

    let mut pixels = vec![0u8; (w * h * 4) as usize];
    let mut put = |x: i32, y: i32, c: [u8; 4]| {
        if x >= 0 && y >= 0 && (x as u32) < w && (y as u32) < h {
            let i = ((y as u32 * w + x as u32) * 4) as usize;
            pixels[i..i + 4].copy_from_slice(&c);
        }
    };
    let cell = CELL as i32;
    let fill = |at: GridPos, color: [u8; 4], put: &mut dyn FnMut(i32, i32, [u8; 4])| {
        for dy in 0..cell {
            for dx in 0..cell {
                put(at.x * cell + dx, at.y * cell + dy, color);
            }
        }
    };
    for at in map.positions() {
        if explored.seen[map.index(at)] {
            let color = match map.get(at) {
                Tile::Wall => WALL,
                Tile::Floor => FLOOR,
                Tile::StairsDown => STAIRS,
            };
            fill(at, color, &mut put);
        }
    }
    // You: a gold cell, with a tick poking out on the side you face.
    fill(p.pos, YOU, &mut put);
    let (fx, fy) = p.facing.delta();
    put(
        p.pos.x * cell + 1 + 2 * fx,
        p.pos.y * cell + 1 + 2 * fy,
        YOU,
    );

    let mut fresh = blank(w, h);
    fresh.data = Some(pixels);
    if let Some(mut img) = images.get_mut(&image.image) {
        *img = fresh;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_cycles_corner_full_hidden() {
        let mut m = MapMode::default();
        assert_eq!(m, MapMode::Hidden);
        let mut seen = Vec::new();
        for _ in 0..4 {
            m = m.next();
            seen.push(m);
        }
        assert_eq!(
            seen,
            [
                MapMode::Corner,
                MapMode::Full,
                MapMode::Hidden,
                MapMode::Corner
            ]
        );
    }

    #[test]
    fn walls_hide_what_is_behind_them() {
        let mut map = Map::filled(9, 3, Tile::Floor);
        map.set(GridPos::new(4, 1), Tile::Wall);
        let me = GridPos::new(2, 1);
        assert!(in_sight(&map, me, GridPos::new(3, 1)));
        // The wall itself is seen; the cell beyond it is not.
        assert!(in_sight(&map, me, GridPos::new(4, 1)));
        assert!(!in_sight(&map, me, GridPos::new(5, 1)));
    }
}
