//! Minimal HUD: Health and Energy bars, the sword in hand, a red flash when
//! hurt, and the death screen. Drawn into the 320x240 frame so it gets the
//! same chunky pixels as the world.

use bevy::prelude::*;
use zrpg_core::Event;

use crate::auto::AutoMode;
use crate::view::{PlayerCamera, ScreenFade};
use crate::{Frame, GameEvent, Run, textures};

const BAR_WIDTH: f32 = 64.0;
const BAR_HEIGHT: f32 = 4.0;
const HEALTH_COLOR: Color = Color::srgb(0.78, 0.16, 0.14);
/// Chi (the core's Energy): jade, so it never reads as gold.
const ENERGY_COLOR: Color = Color::srgb(0.3, 0.78, 0.58);
const GOLD_COLOR: Color = Color::srgb(0.95, 0.75, 0.2);
const EXHAUSTED_COLOR: Color = Color::srgb(1.0, 1.0, 1.0);
const XP_COLOR: Color = Color::srgb(0.45, 0.7, 0.95);
const XP_HEIGHT: f32 = 2.0;
const LEVEL_COLOR: Color = Color::srgb(0.95, 0.85, 0.55);
/// How long a level-up flashes on the level label.
const LEVEL_FLASH_SECS: f32 = 1.2;
const HURT_FLASH_SECS: f32 = 0.3;
const EXHAUSTED_FLASH_SECS: f32 = 0.3;
/// How long the "FLOOR N" banner stays up after descending.
const BANNER_SECS: f32 = 1.6;
/// The sword in hand: twelve whole-screen 320x240 overlays in a 4 x 3
/// sheet, from `art/kits/first-person-sword-v02` (copied by
/// `tools/make_sword.py`). Frame 0 is the sword held at rest.
const SWORD_PNG: &[u8] = include_bytes!("../assets/sword.png");
const SWORD_SHEET: (u32, u32) = (4, 3);
/// One broad swing, high right to low left, then back up: (frame, seconds).
///
/// The kit proposes 830 ms with contact at 240 ms. But a swing lands the
/// instant you press Attack (`zrpg_core`), and the next one is allowed
/// 500 ms later. So the wind-up is cut short, showing contact 110 ms in, and
/// the whole swing fits inside the cooldown; the kit's order of frames and
/// its fast-cut, slower-recovery rhythm are kept.
const SWING: [(usize, f32); 11] = [
    (1, 0.05),  // draw back
    (2, 0.04),  // windup
    (3, 0.02),  // release
    (4, 0.03),  // contact
    (5, 0.03),  // follow-through
    (6, 0.04),  // finish, offscreen
    (7, 0.05),  // lowered reset
    (8, 0.06),  // return low
    (9, 0.05),  // raise
    (10, 0.04), // settle
    (11, 0.04), // idle again
];

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudFx>()
            .add_systems(PostStartup, spawn_hud)
            .add_systems(
                Update,
                (
                    react_to_events,
                    update_hud,
                    update_transition,
                    update_auto_label,
                    update_belt,
                )
                    .chain()
                    .in_set(Frame::Present),
            );
    }
}

#[derive(Resource, Default)]
struct HudFx {
    /// Seconds into the current swing, if swinging.
    swing: Option<f32>,
    hurt: f32,
    exhausted: f32,
    banner: f32,
    level_flash: f32,
}

#[derive(Component)]
struct HealthFill;
#[derive(Component)]
struct EnergyFill;
#[derive(Component)]
struct XpFill;
#[derive(Component)]
struct LevelLabel;
#[derive(Component)]
struct GoldLabel;
/// A belt slot, for the item at this index of `Item::ALL`.
#[derive(Component)]
struct ItemSlot(usize);
#[derive(Component)]
struct ItemCount(usize);
#[derive(Component)]
struct Sword;
#[derive(Component)]
struct HurtFlash;
#[derive(Component)]
struct DeathScreen;
#[derive(Component)]
struct FadeOverlay;
#[derive(Component)]
struct FloorBanner;
#[derive(Component)]
struct AutoLabel;

fn spawn_hud(
    mut commands: Commands,
    cams: Query<Entity, With<PlayerCamera>>,
    loot: Res<crate::loot::LootAssets>,
    mut images: ResMut<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let Ok(cam) = cams.single() else {
        return;
    };
    let sword = images.add(textures::embedded(SWORD_PNG));
    let (cols, rows) = SWORD_SHEET;
    let layout = layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(crate::screen::WIDTH, crate::screen::HEIGHT),
        cols,
        rows,
        None,
        None,
    ));

    let full = || Node {
        position_type: PositionType::Absolute,
        width: percent(100),
        height: percent(100),
        ..default()
    };

    commands
        .spawn((full(), UiTargetCamera(cam)))
        .with_children(|root| {
            // Whole frames at the screen's origin, as the kit asks: its
            // swing leaves the screen on purpose.
            root.spawn((
                Sword,
                ImageNode::from_atlas_image(sword, TextureAtlas { layout, index: 0 }),
                full(),
            ));

            root.spawn(Node {
                position_type: PositionType::Absolute,
                left: px(6),
                bottom: px(6),
                flex_direction: FlexDirection::Column,
                row_gap: px(2),
                ..default()
            })
            .with_children(|bars| {
                // The item belt: one slot per item carried, keyed from 1.
                bars.spawn(Node {
                    column_gap: px(3),
                    margin: UiRect::bottom(px(2)),
                    ..default()
                })
                .with_children(|belt| {
                    for (slot, icon) in loot.icons.iter().enumerate() {
                        belt.spawn((
                            ItemSlot(slot),
                            Node {
                                width: px(crate::loot::ICON as f32 + 4.0),
                                height: px(crate::loot::ICON as f32 + 4.0),
                                padding: UiRect::all(px(1)),
                                border: UiRect::all(px(1)),
                                display: Display::None,
                                ..default()
                            },
                            BorderColor::all(Color::srgba(0.9, 0.8, 0.6, 0.35)),
                            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
                        ))
                        .with_children(|cell| {
                            cell.spawn(ImageNode::new(icon.clone()));
                            let tiny = |text: String, color: Color, place: Node| {
                                (
                                    Text::new(text),
                                    TextFont {
                                        font_size: FontSize::Px(6.0),
                                        ..default()
                                    },
                                    TextColor(color),
                                    shadow(),
                                    place,
                                )
                            };
                            cell.spawn(tiny(
                                format!("{}", slot + 1),
                                Color::srgb(0.75, 0.7, 0.6),
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: px(0),
                                    top: px(0),
                                    ..default()
                                },
                            ));
                            cell.spawn((
                                ItemCount(slot),
                                tiny(
                                    String::new(),
                                    Color::WHITE,
                                    Node {
                                        position_type: PositionType::Absolute,
                                        right: px(0),
                                        bottom: px(-2),
                                        ..default()
                                    },
                                ),
                            ));
                        });
                    }
                });
                bars.spawn(Node {
                    column_gap: px(10),
                    ..default()
                })
                .with_children(|row| {
                    let small = || TextFont {
                        font_size: FontSize::Px(8.0),
                        ..default()
                    };
                    row.spawn((
                        LevelLabel,
                        Text::new("LV 1"),
                        small(),
                        TextColor(LEVEL_COLOR),
                        shadow(),
                    ));
                    row.spawn((
                        GoldLabel,
                        Text::new("0 GOLD"),
                        small(),
                        TextColor(GOLD_COLOR),
                        shadow(),
                    ));
                });
                bar(bars, HealthFill, HEALTH_COLOR, BAR_HEIGHT);
                bar(bars, EnergyFill, ENERGY_COLOR, BAR_HEIGHT);
                bar(bars, XpFill, XP_COLOR, XP_HEIGHT);
            });

            root.spawn((HurtFlash, full(), BackgroundColor(Color::NONE)));
            root.spawn((
                AutoLabel,
                Text::new("AUTO"),
                TextFont {
                    font_size: FontSize::Px(10.0),
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.75, 0.2)),
                Node {
                    position_type: PositionType::Absolute,
                    right: px(6),
                    top: px(4),
                    ..default()
                },
                Visibility::Hidden,
            ));
            root.spawn((FadeOverlay, full(), BackgroundColor(Color::NONE)));
            root.spawn(Node {
                justify_content: JustifyContent::Center,
                top: px(40),
                ..full()
            })
            .with_children(|row| {
                row.spawn((
                    FloorBanner,
                    Text::new(""),
                    TextLayout::justify(Justify::Center),
                    TextFont {
                        font_size: FontSize::Px(16.0),
                        ..default()
                    },
                    TextColor(Color::NONE),
                ));
            });

            root.spawn((
                DeathScreen,
                Node {
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    row_gap: px(6),
                    ..full()
                },
                BackgroundColor(Color::srgba(0.25, 0.0, 0.0, 0.6)),
                Visibility::Hidden,
            ))
            .with_children(|screen| {
                screen.spawn((
                    Text::new("YOU DIED"),
                    TextFont {
                        font_size: FontSize::Px(24.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.15, 0.1)),
                ));
                screen.spawn((
                    Text::new("Attack to rise again"),
                    TextFont {
                        font_size: FontSize::Px(10.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.8, 0.75, 0.7)),
                ));
            });
        });
}

/// A dark drop shadow, so small labels read over bright stone and sky.
fn shadow() -> TextShadow {
    TextShadow {
        offset: Vec2::new(1.0, 1.0),
        color: Color::srgba(0.0, 0.0, 0.0, 0.85),
    }
}

/// A dark, outlined track with a coloured fill whose width tracks a stat.
fn bar(parent: &mut ChildSpawnerCommands, marker: impl Component, color: Color, height: f32) {
    parent
        .spawn((
            Node {
                width: px(BAR_WIDTH + 2.0),
                height: px(height + 2.0),
                border: UiRect::all(px(1)),
                ..default()
            },
            BorderColor::all(Color::srgb(0.05, 0.05, 0.05)),
            BackgroundColor(Color::srgb(0.15, 0.12, 0.12)),
        ))
        .with_children(|track| {
            track.spawn((
                marker,
                Node {
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(color),
            ));
        });
}

/// The sword frame `t` seconds into a swing, or `None` once it is over.
fn swing_frame(t: f32) -> Option<usize> {
    let mut end = 0.0;
    SWING.iter().find_map(|&(frame, secs)| {
        end += secs;
        (t < end).then_some(frame)
    })
}

/// Show the belt slots for items carried, with their counts.
fn update_belt(
    run: Res<Run>,
    mut slots: Query<(&ItemSlot, &mut Node)>,
    mut counts: Query<(&ItemCount, &mut Text)>,
) {
    let bag = &run.player.bag;
    for (slot, mut node) in &mut slots {
        let n = bag.count(zrpg_core::Item::ALL[slot.0]);
        let display = if n > 0 { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
    }
    for (c, mut text) in &mut counts {
        let n = bag.count(zrpg_core::Item::ALL[c.0]);
        let label = if n > 1 { n.to_string() } else { String::new() };
        if text.0 != label {
            text.0 = label;
        }
    }
}

fn react_to_events(mut events: MessageReader<GameEvent>, mut fx: ResMut<HudFx>) {
    for GameEvent(event) in events.read() {
        match event {
            Event::Swung { .. } => fx.swing = Some(0.0),
            Event::EnemyStruck { damage, .. } if *damage > 0 => fx.hurt = HURT_FLASH_SECS,
            Event::Exhausted => fx.exhausted = EXHAUSTED_FLASH_SECS,
            Event::Descended { .. } => fx.banner = BANNER_SECS,
            Event::LeveledUp { .. } => fx.level_flash = LEVEL_FLASH_SECS,
            _ => {}
        }
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn update_hud(
    time: Res<Time>,
    run: Res<Run>,
    auto: Res<AutoMode>,
    hold: crate::Hold,
    map: Res<crate::automap::MapMode>,
    mut fx: ResMut<HudFx>,
    mut xp: Query<&mut Node, (With<XpFill>, Without<HealthFill>, Without<EnergyFill>)>,
    mut level: Query<(&mut Text, &mut TextColor), (With<LevelLabel>, Without<GoldLabel>)>,
    mut gold: Query<&mut Text, (With<GoldLabel>, Without<LevelLabel>)>,
    mut health: Query<
        &mut Node,
        (
            With<HealthFill>,
            Without<EnergyFill>,
            Without<Sword>,
            Without<XpFill>,
        ),
    >,
    mut energy: Query<
        (&mut Node, &mut BackgroundColor),
        (
            With<EnergyFill>,
            Without<HurtFlash>,
            Without<Sword>,
            Without<XpFill>,
        ),
    >,
    mut sword: Query<(&mut ImageNode, &mut Visibility), (With<Sword>, Without<DeathScreen>)>,
    mut flash: Query<&mut BackgroundColor, (With<HurtFlash>, Without<EnergyFill>)>,
    mut death: Query<&mut Visibility, (With<DeathScreen>, Without<Sword>)>,
) {
    let dt = time.delta_secs();
    fx.swing = fx
        .swing
        .map(|t| t + dt)
        .filter(|&t| swing_frame(t).is_some());
    fx.hurt = (fx.hurt - dt).max(0.0);
    fx.exhausted = (fx.exhausted - dt).max(0.0);
    fx.level_flash = (fx.level_flash - dt).max(0.0);
    let p = &run.player;
    for mut n in &mut xp {
        let needed = p.xp_needed();
        let share = if needed == 0 {
            1.0
        } else {
            p.xp as f32 / needed as f32
        };
        n.width = percent(100.0 * share.min(1.0));
    }
    for mut text in &mut gold {
        let label = format!("{} GOLD", p.gold);
        if text.0 != label {
            text.0 = label;
        }
    }
    for (mut text, mut color) in &mut level {
        // In auto mode, level-ups wait for you: show how many.
        let label = match run.pending.len() {
            n if n > 0 && auto.0 => format!("LV {}  +{n}", p.level),
            _ => format!("LV {}", p.level),
        };
        if text.0 != label {
            text.0 = label;
        }
        let flash = fx.level_flash / LEVEL_FLASH_SECS;
        color.0 = LEVEL_COLOR.mix(&Color::WHITE, (flash * 12.0).sin().abs() * flash);
    }
    let alive = p.is_alive();

    for mut n in &mut health {
        n.width = percent(100.0 * p.hp.max(0) as f32 / p.max_hp as f32);
    }
    for (mut n, mut c) in &mut energy {
        n.width = percent(100.0 * p.energy / p.max_energy);
        c.0 = if fx.exhausted > 0.0 {
            EXHAUSTED_COLOR
        } else {
            ENERGY_COLOR
        };
    }
    let frame = fx.swing.and_then(swing_frame).unwrap_or(0);
    for (mut image, mut v) in &mut sword {
        if let Some(atlas) = image.texture_atlas.as_mut()
            && atlas.index != frame
        {
            atlas.index = frame;
        }
        // Put the sword down while choosing an ability or reading the map.
        let busy = hold.covers_the_view() || *map == crate::automap::MapMode::Full;
        *v = if alive && !busy {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for mut c in &mut flash {
        c.0 = Color::srgba(0.8, 0.0, 0.0, 0.45 * fx.hurt / HURT_FLASH_SECS);
    }
    for mut v in &mut death {
        *v = if alive {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}

fn update_transition(
    time: Res<Time>,
    run: Res<Run>,
    fade: Res<ScreenFade>,
    mut fx: ResMut<HudFx>,
    mut overlay: Query<&mut BackgroundColor, With<FadeOverlay>>,
    mut banner: Query<(&mut Text, &mut TextColor), With<FloorBanner>>,
) {
    fx.banner = (fx.banner - time.delta_secs()).max(0.0);
    for mut c in &mut overlay {
        c.0 = Color::srgba(0.0, 0.0, 0.0, fade.0);
    }
    // Hold, then fade out over the last half second.
    let alpha = (fx.banner / 0.5).min(1.0);
    for (mut text, mut color) in &mut banner {
        // A new stage is announced by name.
        let label = match run.floor.stage {
            Some(stage) if stage.first_floor => {
                format!("FLOOR {}\n{}", run.depth, stage.theme.name())
            }
            _ => format!("FLOOR {}", run.depth),
        };
        if text.0 != label {
            text.0 = label;
        }
        color.0 = Color::srgba(0.9, 0.85, 0.75, alpha);
    }
}

fn update_auto_label(
    time: Res<Time>,
    auto: Res<AutoMode>,
    mut label: Query<(&mut Visibility, &mut TextColor), With<AutoLabel>>,
) {
    for (mut v, mut color) in &mut label {
        *v = if auto.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        let pulse = 0.65 + 0.35 * (time.elapsed_secs() * 3.0).sin().abs();
        color.0 = Color::srgba(0.95, 0.75, 0.2, pulse);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zrpg_core::game::tuning::ATTACK_COOLDOWN;

    #[test]
    fn a_swing_shows_contact_early_and_ends_before_the_next() {
        let total: f32 = SWING.iter().map(|s| s.1).sum();
        assert!(
            total < ATTACK_COOLDOWN,
            "swing {total} s outlasts the cooldown"
        );
        assert_eq!(swing_frame(0.0), Some(1));
        let contact = (0..1000)
            .map(|ms| ms as f32 / 1000.0)
            .find(|&t| swing_frame(t) == Some(4));
        assert!(contact.is_some_and(|t| t <= 0.12), "contact at {contact:?}");
        assert_eq!(swing_frame(total + 0.001), None);
    }
}
