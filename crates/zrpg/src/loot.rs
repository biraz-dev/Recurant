//! What the dead leave behind: a pool of gold coins where they vanished,
//! perhaps with an item standing in it, all turning to face you and
//! catching the light until you walk over them. Number keys use items from
//! the bag (see the belt on the HUD).
//!
//! Coins are drawn here, pixel by pixel; item icons are in `assets/items`.

use bevy::asset::RenderAssetUsages;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use zrpg_core::{Command, GridPos, Item};

use crate::atmosphere::{GradedMaterial, Look};
use crate::input::{Action, Actions};
use crate::level::{GroundHeights, cell_center};
use crate::view::PlayerCamera;
use crate::{Frame, GameEvent, Run, publish};

/// The pool's size in world units (a cell is 1.0).
const POOL_SIZE: Vec2 = Vec2::new(0.46, 0.16);
const POOL_W: u32 = 28;
const POOL_H: u32 = 10;
/// Items stand in the pool, this big and this far apart.
const ITEM_SIZE: f32 = 0.2;
const ITEM_SPACING: f32 = 0.16;
/// A soft glow, so loot catches the eye even in the dark.
const GLINT: LinearRgba = LinearRgba::rgb(0.09, 0.06, 0.01);
const ITEM_GLINT: LinearRgba = LinearRgba::rgb(0.05, 0.05, 0.05);
/// Icon pixels.
pub const ICON: u32 = 12;

pub struct LootPlugin;

impl Plugin for LootPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load)
            .add_systems(
                Update,
                use_items.after(crate::auto::toggle).before(Frame::Command),
            )
            .add_systems(
                Update,
                (sync, turn_to_camera).chain().in_set(Frame::Present),
            );
    }
}

#[derive(Resource)]
pub struct LootAssets {
    pool_mesh: Handle<Mesh>,
    pool_texture: Handle<Image>,
    item_mesh: Handle<Mesh>,
    /// One icon per item, in `Item::ALL` order (also used by the HUD).
    pub icons: Vec<Handle<Image>>,
    /// Arrows lying about.
    arrows_icon: Handle<Image>,
}

/// A pool of gold, at a cell.
#[derive(Component)]
struct Pool(GridPos);
/// An item lying at a cell.
#[derive(Component)]
struct Lying(GridPos, Item);
/// Arrows lying at a cell.
#[derive(Component)]
struct Arrows(GridPos);

fn load(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
) {
    let pool_mesh = Mesh::from(Rectangle::new(POOL_SIZE.x, POOL_SIZE.y))
        .translated_by(Vec3::Y * POOL_SIZE.y * 0.5);
    let item_mesh =
        Mesh::from(Rectangle::new(ITEM_SIZE, ITEM_SIZE)).translated_by(Vec3::Y * ITEM_SIZE * 0.5);
    commands.insert_resource(LootAssets {
        pool_mesh: meshes.add(pool_mesh),
        pool_texture: images.add(pool_image()),
        item_mesh: meshes.add(item_mesh),
        icons: Item::ALL.iter().map(|&i| images.add(icon(i))).collect(),
        arrows_icon: images.add(crate::textures::embedded(include_bytes!(
            "../assets/items/arrows.png"
        ))),
    });
}

fn rgba_image(w: u32, h: u32, data: Vec<u8>) -> Image {
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

/// A pool of coins: overlapping gold discs with a dark rim and a bright
/// highlight, spread low and wide, the front ones lowest.
fn pool_image() -> Image {
    // (centre x, centre y, radius) in pixels; drawn back to front.
    const COINS: [(f32, f32, f32); 11] = [
        (7.0, 4.0, 2.8),
        (13.0, 3.6, 2.8),
        (19.5, 4.2, 2.8),
        (10.0, 5.0, 3.0),
        (16.5, 5.2, 3.0),
        (4.0, 6.5, 2.8),
        (23.5, 6.4, 2.8),
        (8.5, 7.2, 3.0),
        (14.0, 7.4, 3.2),
        (19.5, 7.3, 3.0),
        (25.5, 8.0, 2.4),
    ];
    let mut data = vec![0u8; (POOL_W * POOL_H * 4) as usize];
    for (cx, cy, r) in COINS {
        for y in 0..POOL_H {
            for x in 0..POOL_W {
                // Coins lie flat: squash the disc vertically.
                let (dx, dy) = (x as f32 + 0.5 - cx, (y as f32 + 0.5 - cy) * 1.8);
                let d = (dx * dx + dy * dy).sqrt();
                if d > r {
                    continue;
                }
                let rgb: [u8; 3] = if d > r - 0.9 {
                    [122, 74, 18] // rim
                } else if dx < -0.3 && dy < -0.3 && d < r * 0.55 {
                    [255, 240, 160] // highlight
                } else {
                    [236, 178, 52] // face
                };
                let i = ((y * POOL_W + x) * 4) as usize;
                data[i..i + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
            }
        }
    }
    rgba_image(POOL_W, POOL_H, data)
}

/// An item's 12 x 12 icon, from `assets/items` (made by
/// `tools/make_spell_art.py`; repaint freely at the same size).
pub fn icon(item: Item) -> Image {
    let png: &[u8] = match item {
        Item::HealthPotion => include_bytes!("../assets/items/health_potion.png"),
    };
    crate::textures::embedded(png)
}

/// Number keys use the item in that belt slot.
fn use_items(
    actions: Res<Actions>,
    hold: crate::Hold,
    mut run: ResMut<Run>,
    mut writer: MessageWriter<GameEvent>,
    mut buf: Local<Vec<zrpg_core::Event>>,
) {
    if hold.holds_the_game() || !run.player.is_alive() {
        return;
    }
    for (slot, item) in Action::ITEMS.into_iter().zip(Item::ALL) {
        if actions.just_pressed(slot) {
            run.apply(Command::UseItem(item), &mut buf);
        }
    }
    publish(&mut buf, &mut writer);
}

/// The loot's material, and which look (place and stage) it was made for.
struct LootMaterials {
    pool: Handle<GradedMaterial>,
    items: Vec<Handle<GradedMaterial>>,
    arrows: Handle<GradedMaterial>,
    look: (zrpg_core::Place, Option<zrpg_core::Theme>),
}

/// Keep a pool for each pile of gold and a sprite for each item lying.
#[allow(clippy::too_many_arguments)]
fn sync(
    mut commands: Commands,
    run: Res<Run>,
    assets: Res<LootAssets>,
    ground: Res<GroundHeights>,
    mut materials: ResMut<Assets<GradedMaterial>>,
    pools: Query<(Entity, &Pool)>,
    lying: Query<(Entity, &Lying)>,
    bundles: Query<(Entity, &Arrows)>,
    mut cached: Local<Option<LootMaterials>>,
) {
    // Graded like their floor, so loot sits in the same light and haze.
    let key = (run.floor.place, run.floor.stage.map(|s| s.theme));
    if cached.as_ref().is_none_or(|m| m.look != key) {
        let look = Look::of(&run.floor);
        let mut make = |texture: &Handle<Image>, glint, metallic| {
            materials.add(look.graded(StandardMaterial {
                base_color_texture: Some(texture.clone()),
                alpha_mode: AlphaMode::Mask(0.5),
                emissive: glint,
                perceptual_roughness: 0.4,
                metallic,
                cull_mode: None,
                ..default()
            }))
        };
        *cached = Some(LootMaterials {
            pool: make(&assets.pool_texture, GLINT, 0.6),
            items: assets
                .icons
                .iter()
                .map(|i| make(i, ITEM_GLINT, 0.0))
                .collect(),
            arrows: make(&assets.arrows_icon, ITEM_GLINT, 0.2),
            look: key,
        });
    }
    let Some(mats) = cached.as_ref() else {
        return;
    };

    let gold_at = |p: GridPos| run.drops.iter().any(|d| d.pos == p && d.gold > 0);
    let item_at = |p: GridPos, item: Item| {
        run.drops
            .iter()
            .any(|d| d.pos == p && d.items.count(item) > 0)
    };
    for (entity, pool) in &pools {
        if !gold_at(pool.0) {
            commands.entity(entity).despawn();
        }
    }
    for (entity, l) in &lying {
        if !item_at(l.0, l.1) {
            commands.entity(entity).despawn();
        }
    }
    let arrows_at = |p: GridPos| run.drops.iter().any(|d| d.pos == p && d.arrows > 0);
    for (entity, b) in &bundles {
        if !arrows_at(b.0) {
            commands.entity(entity).despawn();
        }
    }
    for drop in &run.drops {
        let at = cell_center(drop.pos);
        let floor = at + Vec3::Y * ground.at(at);
        if drop.gold > 0 && !pools.iter().any(|(_, p)| p.0 == drop.pos) {
            commands.spawn((
                Pool(drop.pos),
                Mesh3d(assets.pool_mesh.clone()),
                MeshMaterial3d(mats.pool.clone()),
                Transform::from_translation(floor),
                NotShadowCaster,
            ));
        }
        // Arrows lie to one side.
        if drop.arrows > 0 && !bundles.iter().any(|(_, b)| b.0 == drop.pos) {
            commands.spawn((
                Arrows(drop.pos),
                Mesh3d(assets.item_mesh.clone()),
                MeshMaterial3d(mats.arrows.clone()),
                Transform::from_translation(floor + Vec3::new(0.16, 0.01, -0.02)),
                NotShadowCaster,
            ));
        }
        // Items stand in a row in the pool, a little behind the coins.
        let present: Vec<Item> = drop.items.items().map(|(i, _)| i).collect();
        for (k, &item) in present.iter().enumerate() {
            if lying.iter().any(|(_, l)| l.0 == drop.pos && l.1 == item) {
                continue;
            }
            let spread = (k as f32 - (present.len() as f32 - 1.0) / 2.0) * ITEM_SPACING;
            commands.spawn((
                Lying(drop.pos, item),
                Mesh3d(assets.item_mesh.clone()),
                MeshMaterial3d(mats.items[item.index()].clone()),
                Transform::from_translation(floor + Vec3::new(spread, 0.02, -0.04)),
                NotShadowCaster,
            ));
        }
    }
}

/// Loot turns to face you, like everything else drawn flat.
#[allow(clippy::type_complexity)]
fn turn_to_camera(
    cams: Query<
        &Transform,
        (
            With<PlayerCamera>,
            Without<Pool>,
            Without<Lying>,
            Without<Arrows>,
        ),
    >,
    mut sprites: Query<&mut Transform, Or<(With<Pool>, With<Lying>, With<Arrows>)>>,
) {
    let Ok(cam) = cams.single() else {
        return;
    };
    let (yaw, _, _) = cam.rotation.to_euler(EulerRot::YXZ);
    for mut t in &mut sprites {
        t.rotation = Quat::from_rotation_y(yaw);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_item_has_an_icon() {
        for item in Item::ALL {
            let img = icon(item);
            assert_eq!((img.width(), img.height()), (ICON, ICON));
            let data = img.data.as_ref().unwrap();
            let opaque = data.chunks(4).filter(|p| p[3] > 0).count();
            assert!(opaque > 20, "{item:?} icon is nearly empty");
        }
    }
}
