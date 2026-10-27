//! Minimal HUD: Health and Energy bars, the sword in hand, a red flash when
//! hurt, and the death screen. Drawn into the 320x240 frame so it gets the
//! same chunky pixels as the world.

use std::f32::consts::PI;

use bevy::prelude::*;
use zrpg_core::Event;

use crate::auto::AutoMode;
use crate::view::{PlayerCamera, ScreenFade};
use crate::{Frame, GameEvent, Run, textures};

const BAR_WIDTH: f32 = 64.0;
const BAR_HEIGHT: f32 = 4.0;
const HEALTH_COLOR: Color = Color::srgb(0.78, 0.16, 0.14);
const ENERGY_COLOR: Color = Color::srgb(0.95, 0.75, 0.2);
const EXHAUSTED_COLOR: Color = Color::srgb(1.0, 1.0, 1.0);
const SWING_SECS: f32 = 0.22;
const HURT_FLASH_SECS: f32 = 0.3;
const EXHAUSTED_FLASH_SECS: f32 = 0.3;
/// How long the "FLOOR N" banner stays up after descending.
const BANNER_SECS: f32 = 1.6;
/// Sword rest position (from the bottom-right corner), in low-res pixels.
const SWORD_RIGHT: f32 = 36.0;
const SWORD_BOTTOM: f32 = -60.0;
const SWORD_SCALE: f32 = 3.0;
/// Resting tilt (tip leaning in towards the centre) and the extra tilt at
/// the peak of a swing, in degrees.
const SWORD_TILT: f32 = 20.0;
const SWING_TILT: f32 = 45.0;

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
                )
                    .chain()
                    .in_set(Frame::Present),
            );
    }
}

#[derive(Resource, Default)]
struct HudFx {
    swing: f32,
    hurt: f32,
    exhausted: f32,
    banner: f32,
}

#[derive(Component)]
struct HealthFill;
#[derive(Component)]
struct EnergyFill;
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
    mut images: ResMut<Assets<Image>>,
) {
    let Ok(cam) = cams.single() else {
        return;
    };
    let sword = images.add(textures::sword());
    let (sw, sh) = textures::SWORD_SIZE;

    let full = || Node {
        position_type: PositionType::Absolute,
        width: percent(100),
        height: percent(100),
        ..default()
    };

    commands
        .spawn((full(), UiTargetCamera(cam)))
        .with_children(|root| {
            root.spawn((
                Sword,
                ImageNode::new(sword),
                UiTransform::from_rotation(Rot2::degrees(SWORD_TILT)),
                Node {
                    position_type: PositionType::Absolute,
                    right: px(SWORD_RIGHT),
                    bottom: px(SWORD_BOTTOM),
                    width: px(sw as f32 * SWORD_SCALE),
                    height: px(sh as f32 * SWORD_SCALE),
                    ..default()
                },
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
                bar(bars, HealthFill, HEALTH_COLOR);
                bar(bars, EnergyFill, ENERGY_COLOR);
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

/// A dark, outlined track with a coloured fill whose width tracks a stat.
fn bar(parent: &mut ChildSpawnerCommands, marker: impl Component, color: Color) {
    parent
        .spawn((
            Node {
                width: px(BAR_WIDTH + 2.0),
                height: px(BAR_HEIGHT + 2.0),
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

fn react_to_events(mut events: MessageReader<GameEvent>, mut fx: ResMut<HudFx>) {
    for GameEvent(event) in events.read() {
        match event {
            Event::Swung { .. } => fx.swing = SWING_SECS,
            Event::EnemyStruck { damage, .. } if *damage > 0 => fx.hurt = HURT_FLASH_SECS,
            Event::Exhausted => fx.exhausted = EXHAUSTED_FLASH_SECS,
            Event::Descended { .. } => fx.banner = BANNER_SECS,
            _ => {}
        }
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn update_hud(
    time: Res<Time>,
    run: Res<Run>,
    mut fx: ResMut<HudFx>,
    mut health: Query<&mut Node, (With<HealthFill>, Without<EnergyFill>, Without<Sword>)>,
    mut energy: Query<
        (&mut Node, &mut BackgroundColor),
        (With<EnergyFill>, Without<HurtFlash>, Without<Sword>),
    >,
    mut sword: Query<
        (&mut Node, &mut UiTransform, &mut Visibility),
        (With<Sword>, Without<DeathScreen>),
    >,
    mut flash: Query<&mut BackgroundColor, (With<HurtFlash>, Without<EnergyFill>)>,
    mut death: Query<&mut Visibility, (With<DeathScreen>, Without<Sword>)>,
) {
    let dt = time.delta_secs();
    fx.swing = (fx.swing - dt).max(0.0);
    fx.hurt = (fx.hurt - dt).max(0.0);
    fx.exhausted = (fx.exhausted - dt).max(0.0);
    let p = &run.player;
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
    for (mut n, mut tf, mut v) in &mut sword {
        // Swing: chop up, across and in towards the centre, then settle back.
        let s = (fx.swing / SWING_SECS * PI).sin().max(0.0);
        n.right = px(SWORD_RIGHT + 80.0 * s);
        n.bottom = px(SWORD_BOTTOM + 36.0 * s);
        tf.rotation = Rot2::degrees(SWORD_TILT + SWING_TILT * s);
        *v = if alive {
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
        let label = format!("FLOOR {}", run.depth);
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
