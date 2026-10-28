//! The equipment screen: an old paper doll. A figure stands in a lit
//! alcove, ringed by the nine slots it can wear (helmet, necklace, armor,
//! both hands, gloves, belt, two rings); beside it, your stats and what the
//! chosen slot holds. I (gamepad Start) opens and closes it; the world holds
//! still while it is open. Arrows (or WASD) move between slots.
//!
//! The art is in `assets/equipment`, made by `tools/make_equipment_art.py`:
//! placeholder pixels, meant to be repainted at the same sizes. The slots
//! sit where `SLOTS` puts them, over the backdrop's alcove.

use bevy::prelude::*;
use zrpg_core::{Gear, Slot};

use crate::input::{Action, Actions};
use crate::view::PlayerCamera;
use crate::{Run, textures};

const BACKDROP_PNG: &[u8] = include_bytes!("../assets/equipment/backdrop.png");
const FIGURE_PNG: &[u8] = include_bytes!("../assets/equipment/figure.png");
const SLOT_PNG: &[u8] = include_bytes!("../assets/equipment/slot.png");

/// Top-left corner of each slot on the 320 x 240 screen, in `Slot::ALL`
/// order. Your right hand is on the left, as when facing someone.
const SLOTS: [(f32, f32); 9] = [
    (92.0, 8.0),    // helmet
    (160.0, 52.0),  // necklace
    (24.0, 52.0),   // armor
    (24.0, 92.0),   // right hand
    (160.0, 92.0),  // left hand
    (24.0, 132.0),  // gloves
    (160.0, 132.0), // belt
    (24.0, 172.0),  // ring (right hand)
    (160.0, 172.0), // ring (left hand)
];
const SLOT_SIZE: f32 = 20.0;
const FIGURE_AT: (f32, f32) = (60.0, 34.0);
/// The stats panel's inside.
const PANEL: (f32, f32, f32) = (207.0, 13.0, 100.0);

const GOLD: Color = Color::srgb(0.95, 0.75, 0.2);
const INK: Color = Color::srgb(0.9, 0.86, 0.78);
const DIM: Color = Color::srgb(0.62, 0.58, 0.52);
const LABEL: Color = Color::srgb(0.7, 0.62, 0.48);

pub struct EquipmentPlugin;

impl Plugin for EquipmentPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Sheet>()
            .add_systems(PostStartup, spawn)
            .add_systems(
                Update,
                (drive, show)
                    .chain()
                    .after(crate::picker::drive)
                    .before(crate::Frame::Command),
            );
    }
}

/// Whether the screen is open, and which slot is chosen.
#[derive(Resource, Default)]
pub struct Sheet {
    open: bool,
    selected: usize,
}

impl Sheet {
    pub fn is_open(&self) -> bool {
        self.open
    }
}

#[derive(Component)]
struct Root;
/// The icon in a slot: what it holds, or a faint outline when empty.
#[derive(Component)]
struct SlotIcon(usize);
#[derive(Component)]
struct Cursor;
#[derive(Component)]
struct Stats;
#[derive(Component)]
struct ChosenSlot;
#[derive(Component)]
struct ChosenGear;
#[derive(Component)]
struct ChosenText;

#[derive(Resource)]
struct Icons {
    empty: Vec<Handle<Image>>,
    gear: Vec<Handle<Image>>,
}

/// The faint outline for each empty slot, in `Slot::ALL` order.
fn empty_png(slot: Slot) -> &'static [u8] {
    match slot {
        Slot::Helmet => include_bytes!("../assets/equipment/empty/helmet.png"),
        Slot::Necklace => include_bytes!("../assets/equipment/empty/necklace.png"),
        Slot::Armor => include_bytes!("../assets/equipment/empty/armor.png"),
        Slot::RightHand => include_bytes!("../assets/equipment/empty/right_hand.png"),
        Slot::LeftHand => include_bytes!("../assets/equipment/empty/left_hand.png"),
        Slot::Gloves => include_bytes!("../assets/equipment/empty/gloves.png"),
        Slot::Belt => include_bytes!("../assets/equipment/empty/belt.png"),
        Slot::RingRight | Slot::RingLeft => include_bytes!("../assets/equipment/empty/ring.png"),
    }
}

fn gear_png(gear: Gear) -> &'static [u8] {
    match gear {
        Gear::Sword => include_bytes!("../assets/equipment/gear/sword.png"),
        Gear::Torch => include_bytes!("../assets/equipment/gear/torch.png"),
    }
}

fn font(size: f32) -> TextFont {
    TextFont {
        font_size: FontSize::Px(size),
        ..default()
    }
}

fn at(x: f32, y: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: px(x),
        top: px(y),
        ..default()
    }
}

fn spawn(
    mut commands: Commands,
    cams: Query<Entity, With<PlayerCamera>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Ok(cam) = cams.single() else {
        return;
    };
    let mut load = |png: &[u8]| images.add(textures::embedded(png));
    let icons = Icons {
        empty: Slot::ALL.iter().map(|&s| load(empty_png(s))).collect(),
        gear: Gear::ALL.iter().map(|&g| load(gear_png(g))).collect(),
    };
    let (backdrop, figure, frame) = (load(BACKDROP_PNG), load(FIGURE_PNG), load(SLOT_PNG));

    commands
        .spawn((
            Root,
            UiTargetCamera(cam),
            // Over the HUD and the map; the ability picker never opens with it.
            GlobalZIndex(8),
            ImageNode::new(backdrop),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                ..default()
            },
            Visibility::Hidden,
        ))
        .with_children(|root| {
            root.spawn((ImageNode::new(figure), at(FIGURE_AT.0, FIGURE_AT.1)));
            for (i, &(x, y)) in SLOTS.iter().enumerate() {
                root.spawn((
                    ImageNode::new(frame.clone()),
                    Node {
                        width: px(SLOT_SIZE),
                        height: px(SLOT_SIZE),
                        padding: UiRect::all(px(2)),
                        ..at(x, y)
                    },
                ))
                .with_children(|slot| {
                    slot.spawn((SlotIcon(i), ImageNode::new(icons.empty[i].clone())));
                });
                root.spawn((
                    Text::new(Slot::ALL[i].name()),
                    font(6.0),
                    TextColor(LABEL),
                    TextLayout::justify(Justify::Center),
                    Node {
                        width: px(48),
                        ..at(x + SLOT_SIZE / 2.0 - 24.0, y + SLOT_SIZE + 1.0)
                    },
                ));
            }
            root.spawn((
                Cursor,
                Node {
                    width: px(SLOT_SIZE + 4.0),
                    height: px(SLOT_SIZE + 4.0),
                    border: UiRect::all(px(1)),
                    ..at(SLOTS[0].0 - 2.0, SLOTS[0].1 - 2.0)
                },
                BorderColor::all(GOLD),
            ));

            let (px0, py0, width) = PANEL;
            root.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(3),
                width: px(width),
                ..at(px0, py0)
            })
            .with_children(|panel| {
                panel.spawn((Text::new("CHARACTER"), font(10.0), TextColor(GOLD)));
                panel.spawn((Stats, Text::new(""), font(7.0), TextColor(INK)));
                panel.spawn((
                    Node {
                        height: px(1),
                        margin: UiRect::vertical(px(3)),
                        ..default()
                    },
                    BackgroundColor(DIM.with_alpha(0.5)),
                ));
                panel.spawn((ChosenSlot, Text::new(""), font(8.0), TextColor(GOLD)));
                panel.spawn((ChosenGear, Text::new(""), font(8.0), TextColor(INK)));
                panel.spawn((ChosenText, Text::new(""), font(7.0), TextColor(DIM)));
            });
            root.spawn((
                Text::new("ARROWS  choose\nI  close"),
                font(6.0),
                TextColor(DIM),
                at(px0, 206.0),
            ));
        });
    commands.insert_resource(icons);
}

/// I opens and closes; arrows move between slots.
fn drive(
    actions: Res<Actions>,
    picker: Res<crate::picker::Picker>,
    run: Res<Run>,
    mut sheet: ResMut<Sheet>,
) {
    if picker.is_open() || !run.player.is_alive() {
        sheet.open = false;
        return;
    }
    if actions.just_pressed(Action::Gear) {
        sheet.open = !sheet.open;
        return;
    }
    if !sheet.open {
        return;
    }
    let pressed = |a: &[Action]| a.iter().any(|&a| actions.just_pressed(a));
    let dir = if pressed(&[Action::Forward]) {
        Vec2::NEG_Y
    } else if pressed(&[Action::Back]) {
        Vec2::Y
    } else if pressed(&[Action::TurnLeft, Action::StrafeLeft]) {
        Vec2::NEG_X
    } else if pressed(&[Action::TurnRight, Action::StrafeRight]) {
        Vec2::X
    } else {
        return;
    };
    sheet.selected = neighbour(sheet.selected, dir);
}

/// The slot to move to from `from` in direction `dir`: the nearest one that
/// way, favouring those straight ahead. Stays put at the edge.
fn neighbour(from: usize, dir: Vec2) -> usize {
    let here = Vec2::from(SLOTS[from]);
    SLOTS
        .iter()
        .enumerate()
        .filter_map(|(i, &p)| {
            let d = Vec2::from(p) - here;
            let along = d.dot(dir);
            (i != from && along > 0.0).then(|| (i, along + 2.0 * d.perp_dot(dir).abs()))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map_or(from, |(i, _)| i)
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn show(
    sheet: Res<Sheet>,
    run: Res<Run>,
    icons: Option<Res<Icons>>,
    mut root: Query<&mut Visibility, With<Root>>,
    mut slot_icons: Query<(&SlotIcon, &mut ImageNode)>,
    mut cursor: Query<&mut Node, With<Cursor>>,
    mut texts: Query<(
        &mut Text,
        Has<Stats>,
        Has<ChosenSlot>,
        Has<ChosenGear>,
        Has<ChosenText>,
    )>,
) {
    let want = if sheet.open {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut v in &mut root {
        if *v != want {
            *v = want;
        }
    }
    let Some(icons) = icons else {
        return;
    };
    if !sheet.open {
        return;
    }
    let p = &run.player;
    let kit = &p.equipment;
    for (slot, mut image) in &mut slot_icons {
        let icon = match kit.get(Slot::ALL[slot.0]) {
            Some(gear) => &icons.gear[gear.index()],
            None => &icons.empty[slot.0],
        };
        if image.image != *icon {
            image.image = icon.clone();
        }
    }
    let (x, y) = SLOTS[sheet.selected];
    for mut node in &mut cursor {
        let (left, top) = (px(x - 2.0), px(y - 2.0));
        if node.left != left || node.top != top {
            node.left = left;
            node.top = top;
        }
    }

    let (low, high) = p.swing_damage();
    let stats = format!(
        "LEVEL {}\nHEALTH {} / {}\nCHI {} / {}\nGOLD {}\nSWING {}-{}\nSPELLS +{}\nABILITIES {}",
        p.level,
        p.hp,
        p.max_hp,
        p.energy.floor() as i32,
        p.max_energy as i32,
        p.gold,
        low,
        high,
        zrpg_core::spells::level_bonus(p.level),
        zrpg_core::Perk::ALL
            .iter()
            .map(|&k| p.perks.count(k) as u32)
            .sum::<u32>(),
    );
    let slot = Slot::ALL[sheet.selected];
    let gear = kit.get(slot);
    for (mut text, is_stats, is_slot, is_gear, is_text) in &mut texts {
        let label = if is_stats {
            stats.clone()
        } else if is_slot {
            slot.name().to_string()
        } else if is_gear {
            gear.map_or("Empty".to_string(), |g| g.name().to_string())
        } else if is_text {
            gear.map_or("Nothing worn here yet.", |g| g.description())
                .to_string()
        } else {
            continue;
        };
        if text.0 != label {
            text.0 = label;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrows_reach_every_slot() {
        // From the helmet, walking the screen reaches all nine slots.
        let mut seen = vec![false; SLOTS.len()];
        let mut stack = vec![0];
        while let Some(i) = stack.pop() {
            if std::mem::replace(&mut seen[i], true) {
                continue;
            }
            for dir in [Vec2::X, Vec2::NEG_X, Vec2::Y, Vec2::NEG_Y] {
                stack.push(neighbour(i, dir));
            }
        }
        assert!(seen.iter().all(|&s| s), "{seen:?}");
    }

    #[test]
    fn arrows_move_the_obvious_way() {
        let armor = Slot::Armor.index();
        assert_eq!(neighbour(armor, Vec2::X), Slot::Necklace.index());
        assert_eq!(neighbour(armor, Vec2::Y), Slot::RightHand.index());
        assert_eq!(neighbour(armor, Vec2::NEG_Y), Slot::Helmet.index());
        assert_eq!(neighbour(armor, Vec2::NEG_X), armor);
    }
}
