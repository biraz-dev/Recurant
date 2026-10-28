//! The bow, and everything else that flies. V (gamepad West) looses an
//! arrow down your line of sight; the rules are in `zrpg_core::archery`.
//! This draws whatever is in the air, leaving from the off hand and flying
//! true down the line: arrows, magic hammers (tumbling end over end, out
//! and home again) and barrage sparks (a spray of little glowing motes).
//! It also keeps the bow's slot on the HUD (left of the spells) with the
//! arrows left.
//!
//! The arrow and hammer are built from a few simple shapes here; the icons
//! are in
//! `assets/equipment/gear` and `assets/items`, made by
//! `tools/make_archery_art.py`.

use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use zrpg_core::archery::{Missile, tuning};
use zrpg_core::{Command, Event};

use crate::atmosphere::Look;
use crate::input::{Action, Actions};
use crate::level::{GroundHeights, cell_center};
use crate::view::PlayerCamera;
use crate::{Frame, GameEvent, Run, publish, textures};

const BOW_PNG: &[u8] = include_bytes!("../assets/equipment/gear/bow.png");
const ICON: f32 = 16.0;
/// Arrows fly a little below the eye.
const FLIGHT_DROP: f32 = 0.1;
/// They leave from the off hand (left and low), joining the line of sight
/// over the first cell.
const HAND_OFFSET: Vec2 = Vec2::new(-0.12, -0.08);
/// Hammers tumble this fast (radians per cell flown).
const HAMMER_SPIN: f32 = 5.0;
/// Barrage sparks scatter this far about the line, so the stream sprays.
const SPARK_SPRAY: f32 = 0.07;

pub struct ArcheryPlugin;

impl Plugin for ArcheryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load)
            .add_systems(PostStartup, spawn_slot)
            .add_systems(
                Update,
                shoot.after(crate::auto::toggle).before(Frame::Command),
            )
            .add_systems(Update, (fly, update_slot).in_set(Frame::Present));
    }
}

#[derive(Resource)]
struct ArrowAssets {
    shaft: Handle<Mesh>,
    head: Handle<Mesh>,
    vane_upright: Handle<Mesh>,
    vane_flat: Handle<Mesh>,
    wood: Handle<StandardMaterial>,
    iron: Handle<StandardMaterial>,
    feather: Handle<StandardMaterial>,
    hammer_head: Handle<Mesh>,
    hammer_haft: Handle<Mesh>,
    hammer_metal: Handle<StandardMaterial>,
    spark: Handle<Mesh>,
    spark_glow: Handle<StandardMaterial>,
    bow_icon: Handle<Image>,
}

/// An arrow in the air, following the core's flight with this id.
#[derive(Component)]
struct ArrowInFlight(u32);

fn load(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let matte = |color: Color| StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.8,
        ..default()
    };
    commands.insert_resource(ArrowAssets {
        shaft: meshes.add(Cuboid::new(0.016, 0.016, 0.5)),
        head: meshes.add(Cone {
            radius: 0.026,
            height: 0.07,
        }),
        vane_upright: meshes.add(Cuboid::new(0.003, 0.045, 0.1)),
        vane_flat: meshes.add(Cuboid::new(0.045, 0.003, 0.1)),
        wood: materials.add(matte(Color::srgb(0.62, 0.42, 0.22))),
        iron: materials.add(StandardMaterial {
            base_color: Color::srgb(0.6, 0.62, 0.68),
            metallic: 0.7,
            perceptual_roughness: 0.35,
            ..default()
        }),
        feather: materials.add(StandardMaterial {
            cull_mode: None,
            ..matte(Color::srgb(0.78, 0.25, 0.2))
        }),
        hammer_head: meshes.add(Cuboid::new(0.16, 0.09, 0.09)),
        hammer_haft: meshes.add(Cuboid::new(0.025, 0.22, 0.025)),
        // Cold, enchanted silver: it glows a little on its own.
        hammer_metal: materials.add(StandardMaterial {
            base_color: Color::srgb(0.78, 0.82, 0.92),
            emissive: LinearRgba::rgb(0.25, 0.35, 0.6),
            metallic: 0.8,
            perceptual_roughness: 0.3,
            ..default()
        }),
        spark: meshes.add(Sphere::new(0.03)),
        spark_glow: materials.add(StandardMaterial {
            base_color: Color::LinearRgba(LinearRgba::rgb(2.2, 2.4, 2.8)),
            unlit: true,
            fog_enabled: false,
            ..default()
        }),
        bow_icon: images.add(textures::embedded(BOW_PNG)),
    });
}

/// V shoots.
fn shoot(
    actions: Res<Actions>,
    hold: crate::Hold,
    mut run: ResMut<Run>,
    mut writer: MessageWriter<GameEvent>,
    mut buf: Local<Vec<Event>>,
) {
    if hold.holds_the_game() || !run.player.is_alive() {
        return;
    }
    if actions.just_pressed(Action::Shoot) {
        run.apply(Command::Shoot, &mut buf);
    }
    publish(&mut buf, &mut writer);
}

/// Keep one missile for each flight in the core, where it has got to.
fn fly(
    mut commands: Commands,
    run: Res<Run>,
    assets: Res<ArrowAssets>,
    ground: Res<GroundHeights>,
    arrows: Query<(Entity, &ArrowInFlight)>,
    mut transforms: Query<&mut Transform, With<ArrowInFlight>>,
) {
    for (entity, arrow) in &arrows {
        if !run.flights.iter().any(|f| f.id == arrow.0) {
            commands.entity(entity).despawn();
        }
    }
    let eye = Look::of(&run.floor).eye_height - FLIGHT_DROP;
    for f in &run.flights {
        let (dx, dz) = f.dir.delta();
        let dir = Vec3::new(dx as f32, 0.0, dz as f32);
        let right = dir.cross(Vec3::Y);
        // Off the hand at first, onto the line of sight within a cell.
        let join = (1.0 - f.dist).max(0.0);
        let along = cell_center(f.from) + dir * f.dist;
        let mut at = along
            + right * HAND_OFFSET.x * join
            + Vec3::Y * (ground.at(along) + eye + HAND_OFFSET.y * join);
        let mut place = Transform::from_translation(at).looking_to(dir, Vec3::Y);
        match f.missile {
            Missile::Arrow => {}
            Missile::Hammer => {
                // End over end, the same way out and home.
                place.rotate_local_x(-f.dist * HAMMER_SPIN);
            }
            Missile::Spark => {
                // Each spark strays a little from the line, its own way.
                let h = zrpg_core::rng::mix64(f.id as u64);
                let side = (h % 1000) as f32 / 500.0 - 1.0;
                let lift = (h / 1000 % 1000) as f32 / 500.0 - 1.0;
                let spread = SPARK_SPRAY * (f.dist / 2.0).min(1.0);
                at += right * side * spread + Vec3::Y * lift * spread;
                place.translation = at;
            }
        }
        if let Some((entity, _)) = arrows.iter().find(|(_, a)| a.0 == f.id) {
            if let Ok(mut t) = transforms.get_mut(entity) {
                *t = place;
            }
            continue;
        }
        let mut flying = commands.spawn((ArrowInFlight(f.id), place, Visibility::default()));
        match f.missile {
            Missile::Arrow => {}
            Missile::Hammer => {
                flying.with_children(|hammer| {
                    hammer.spawn((
                        Mesh3d(assets.hammer_head.clone()),
                        MeshMaterial3d(assets.hammer_metal.clone()),
                        Transform::from_xyz(0.0, 0.1, 0.0),
                        NotShadowCaster,
                    ));
                    hammer.spawn((
                        Mesh3d(assets.hammer_haft.clone()),
                        MeshMaterial3d(assets.wood.clone()),
                        NotShadowCaster,
                    ));
                    hammer.spawn(PointLight {
                        color: Color::srgb(0.6, 0.75, 1.0),
                        intensity: 2500.0,
                        range: 2.5,
                        radius: 0.05,
                        shadow_maps_enabled: false,
                        ..default()
                    });
                });
                continue;
            }
            Missile::Spark => {
                flying.insert((
                    Mesh3d(assets.spark.clone()),
                    MeshMaterial3d(assets.spark_glow.clone()),
                    NotShadowCaster,
                ));
                continue;
            }
        }
        flying.with_children(|arrow| {
            // Forward is -Z: head in front, vanes at the back.
            arrow.spawn((
                Mesh3d(assets.shaft.clone()),
                MeshMaterial3d(assets.wood.clone()),
                NotShadowCaster,
            ));
            arrow.spawn((
                Mesh3d(assets.head.clone()),
                MeshMaterial3d(assets.iron.clone()),
                Transform::from_xyz(0.0, 0.0, -0.28)
                    .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
                NotShadowCaster,
            ));
            for vane in [&assets.vane_upright, &assets.vane_flat] {
                arrow.spawn((
                    Mesh3d(vane.clone()),
                    MeshMaterial3d(assets.feather.clone()),
                    Transform::from_xyz(0.0, 0.0, 0.2),
                    NotShadowCaster,
                ));
            }
        });
    }
}

#[derive(Component)]
struct BowSlot;
#[derive(Component)]
struct BowIcon;
#[derive(Component)]
struct BowShade;
#[derive(Component)]
struct ArrowCount;

/// The bow's slot, left of the spell bar: the bow, its key, the arrows left.
fn spawn_slot(
    mut commands: Commands,
    cams: Query<Entity, With<PlayerCamera>>,
    assets: Res<ArrowAssets>,
) {
    let Ok(cam) = cams.single() else {
        return;
    };
    // The spell slots (ICON + 4 each, with gaps of 3), then a little room.
    let spells = zrpg_core::Spell::ALL.len() as f32;
    let right = 6.0 + spells * (ICON + 4.0) + (spells - 1.0) * 3.0 + 6.0;
    let tiny = |text: &str, color: Color, place: Node| {
        (
            Text::new(text),
            TextFont {
                font_size: FontSize::Px(6.0),
                ..default()
            },
            TextColor(color),
            TextShadow {
                offset: Vec2::new(1.0, 1.0),
                color: Color::srgba(0.0, 0.0, 0.0, 0.85),
            },
            place,
        )
    };
    commands
        .spawn((
            BowSlot,
            UiTargetCamera(cam),
            Node {
                position_type: PositionType::Absolute,
                right: px(right),
                bottom: px(6),
                width: px(ICON + 4.0),
                height: px(ICON + 4.0),
                padding: UiRect::all(px(1)),
                border: UiRect::all(px(1)),
                ..default()
            },
            BorderColor::all(Color::srgba(0.9, 0.75, 0.5, 0.45)),
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
        ))
        .with_children(|slot| {
            slot.spawn((BowIcon, ImageNode::new(assets.bow_icon.clone())));
            slot.spawn((
                BowShade,
                Node {
                    position_type: PositionType::Absolute,
                    left: px(1),
                    right: px(1),
                    bottom: px(1),
                    height: px(0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            ));
            slot.spawn(tiny(
                "V",
                Color::srgb(0.95, 0.85, 0.65),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    top: px(0),
                    ..default()
                },
            ));
            slot.spawn((
                ArrowCount,
                tiny(
                    "",
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

/// Arrows left; dim with none; shade while the bow is drawn again.
fn update_slot(
    run: Res<Run>,
    mut icon: Query<&mut ImageNode, With<BowIcon>>,
    mut shade: Query<&mut Node, With<BowShade>>,
    mut count: Query<&mut Text, With<ArrowCount>>,
) {
    let p = &run.player;
    for mut image in &mut icon {
        let tint = if p.arrows > 0 {
            Color::WHITE
        } else {
            Color::srgba(0.45, 0.45, 0.55, 0.8)
        };
        if image.color != tint {
            image.color = tint;
        }
    }
    let h = px(ICON * (p.bow_cooldown / tuning::COOLDOWN).clamp(0.0, 1.0));
    for mut node in &mut shade {
        if node.height != h {
            node.height = h;
        }
    }
    let label = p.arrows.to_string();
    for mut text in &mut count {
        if text.0 != label {
            text.0 = label.clone();
        }
    }
}
