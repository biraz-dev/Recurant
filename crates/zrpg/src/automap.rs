//! The automap: a small, faint map in the top-left corner that fills in as
//! you explore. It shows only what you have seen (cells within a few steps
//! with a clear line of sight), so the unexplored stays dark. Creatures are
//! not on it.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use zrpg_core::{Facing, GridPos, Map, Tile};

use crate::view::PlayerCamera;
use crate::{Frame, Run};

/// Screen pixels per map cell.
const CELL: u32 = 3;
/// How far you can see to map, in cells.
const SIGHT: i32 = 4;
/// The whole map is drawn this faint.
const OPACITY: f32 = 0.75;
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
            .add_systems(PostStartup, spawn)
            .add_systems(Update, (explore, draw).chain().in_set(Frame::Present));
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
    commands.spawn((
        AutomapImage,
        UiTargetCamera(cam),
        ImageNode::new(image).with_color(Color::srgba(1.0, 1.0, 1.0, OPACITY)),
        Node {
            position_type: PositionType::Absolute,
            left: px(5),
            top: px(5),
            ..default()
        },
    ));
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

fn draw(
    run: Res<Run>,
    picker: Res<crate::picker::Picker>,
    mut explored: ResMut<Explored>,
    mut images: ResMut<Assets<Image>>,
    mut nodes: Query<(&ImageNode, &mut Node, &mut Visibility), With<AutomapImage>>,
) {
    let Ok((image, mut node, mut vis)) = nodes.single_mut() else {
        return;
    };
    let show = run.player.is_alive() && !picker.is_open();
    let want = if show {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if *vis != want {
        *vis = want;
    }
    let p = &run.player;
    let seen_count = explored.seen.iter().filter(|s| **s).count();
    let state = Some((p.pos, p.facing, seen_count));
    if explored.drawn == state {
        return;
    }
    explored.drawn = state;

    let map = &run.floor.map;
    let (w, h) = (map.width() as u32 * CELL, map.height() as u32 * CELL);
    let mut pixels = vec![0u8; (w * h * 4) as usize];
    let mut put = |x: u32, y: u32, c: [u8; 4]| {
        if x < w && y < h {
            let i = ((y * w + x) * 4) as usize;
            pixels[i..i + 4].copy_from_slice(&c);
        }
    };
    for cell in map.positions() {
        if !explored.seen[map.index(cell)] {
            continue;
        }
        let color = match map.get(cell) {
            Tile::Wall => WALL,
            Tile::Floor => FLOOR,
            Tile::StairsDown => STAIRS,
        };
        let (x0, y0) = (cell.x as u32 * CELL, cell.y as u32 * CELL);
        for dy in 0..CELL {
            for dx in 0..CELL {
                put(x0 + dx, y0 + dy, color);
            }
        }
    }
    // You: a gold cell, with a tick poking out on the side you face.
    let (x0, y0) = (p.pos.x as u32 * CELL, p.pos.y as u32 * CELL);
    for dy in 0..CELL {
        for dx in 0..CELL {
            put(x0 + dx, y0 + dy, YOU);
        }
    }
    let (fx, fy) = p.facing.delta();
    let (cx, cy) = (x0 as i32 + 1, y0 as i32 + 1);
    put((cx + 2 * fx) as u32, (cy + 2 * fy) as u32, YOU);

    let mut fresh = blank(w, h);
    fresh.data = Some(pixels);
    if let Some(mut img) = images.get_mut(&image.image) {
        *img = fresh;
    }
    node.width = px(w as f32);
    node.height = px(h as f32);
}

#[cfg(test)]
mod tests {
    use super::*;

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
