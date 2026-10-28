//! Spells in hand. Z, X and C cast Spark, Chain Lightning and Lightning
//! Bolt (the rules are in `zrpg_core::spells`); this draws them: jagged arcs
//! of light from the hand, little bursts of sparks where they strike, and a
//! blue flash on the walls. The spell bar sits bottom right.
//!
//! The art is in `assets/spells`, made by `tools/make_spell_art.py`:
//! placeholder pixels, meant to be repainted at the same sizes.

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use zrpg_core::{Command, Event, GridPos, Spell};

use crate::enemies::EnemySprite;
use crate::input::{Action, Actions};
use crate::level::{GroundHeights, cell_center};
use crate::view::PlayerCamera;
use crate::{Frame, GameEvent, Run, publish, textures};

const LIGHTNING_PNG: &[u8] = include_bytes!("../assets/spells/lightning.png");
const SPARKS_PNG: &[u8] = include_bytes!("../assets/spells/sparks.png");
const ICON_PNGS: [&[u8]; 3] = [
    include_bytes!("../assets/spells/spark.png"),
    include_bytes!("../assets/spells/chain.png"),
    include_bytes!("../assets/spells/bolt.png"),
];
const KEYS: [&str; 3] = ["Z", "X", "C"];
const SPARK_FRAMES: u32 = 4;
const ICON: f32 = 16.0;

/// Where spells leave the body, in camera space: the off hand, low and to
/// the left, clear of the sword.
const HAND: Vec3 = Vec3::new(-0.11, -0.1, -0.3);
/// Where on a body spells strike, above its feet.
const CHEST: f32 = 0.5;
/// Seconds between one Spark's strikes, and between a chain's jumps.
const SPARK_GAP: f32 = 0.09;
const CHAIN_GAP: f32 = 0.08;
/// Blue-white light, shared by arcs, sparks and flashes.
const SPELL_LIGHT: Color = Color::srgb(0.6, 0.85, 1.0);
const FLASH_LUMENS: f32 = 9_000.0;
const FLASH_RANGE: f32 = 4.0;
const FLASH_SECS: f32 = 0.18;
/// How far along an arc it takes to grow to its full width.
const TAPER: f32 = 1.2;
/// How often an arc re-jags while it lasts.
const JITTER_SECS: f32 = 0.04;

pub struct SpellsPlugin;

impl Plugin for SpellsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Queue>()
            .add_systems(Startup, load)
            .add_systems(PostStartup, spawn_bar)
            .add_systems(
                Update,
                cast.after(crate::auto::toggle).before(Frame::Command),
            )
            .add_systems(
                Update,
                (
                    queue_effects,
                    play_queue,
                    animate_arcs,
                    animate_bursts,
                    fade_flashes,
                    update_bar,
                )
                    .chain()
                    .in_set(Frame::Present),
            );
    }
}

#[derive(Resource)]
struct SpellAssets {
    lightning: Handle<Image>,
    /// One quad of spark frames each.
    sparks: Vec<Handle<StandardMaterial>>,
    spark_mesh: Handle<Mesh>,
    icons: Vec<Handle<Image>>,
}

fn load(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut lightning = textures::embedded(LIGHTNING_PNG);
    // The strip repeats along an arc's length.
    lightning.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        ..ImageSamplerDescriptor::nearest()
    });
    let sheet = images.add(textures::embedded(SPARKS_PNG));
    let step = 1.0 / SPARK_FRAMES as f32;
    let sparks = (0..SPARK_FRAMES)
        .map(|f| {
            materials.add(StandardMaterial {
                base_color_texture: Some(sheet.clone()),
                uv_transform: bevy::math::Affine2::from_scale_angle_translation(
                    Vec2::new(step, 1.0),
                    0.0,
                    Vec2::new(f as f32 * step, 0.0),
                ),
                ..glow(SPELL_LIGHT)
            })
        })
        .collect();
    commands.insert_resource(SpellAssets {
        lightning: images.add(lightning),
        sparks,
        spark_mesh: meshes.add(Rectangle::new(0.14, 0.14)),
        icons: ICON_PNGS
            .iter()
            .map(|png| images.add(textures::embedded(png)))
            .collect(),
    });
}

/// Light laid over what is behind it, untouched by fog or shading. Blended
/// rather than added, so it still shows against sunlit stone.
fn glow(color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        fog_enabled: false,
        cull_mode: None,
        ..default()
    }
}

/// Z, X and C cast.
fn cast(
    actions: Res<Actions>,
    picker: Res<crate::picker::Picker>,
    mut run: ResMut<Run>,
    mut writer: MessageWriter<GameEvent>,
    mut buf: Local<Vec<Event>>,
) {
    if picker.holds_the_game() || !run.player.is_alive() {
        return;
    }
    for (key, spell) in Action::SPELLS.into_iter().zip(Spell::ALL) {
        if actions.just_pressed(key) {
            run.apply(Command::Cast(spell), &mut buf);
        }
    }
    publish(&mut buf, &mut writer);
}

/// How an arc looks.
#[derive(Clone, Copy)]
struct Style {
    width: f32,
    /// How far it wanders from a straight line, per unit of length.
    jag: f32,
    secs: f32,
    brightness: f32,
}

const SPARK_ARC: Style = Style {
    width: 0.05,
    jag: 0.12,
    secs: 0.1,
    brightness: 1.1,
};
const CHAIN_ARC: Style = Style {
    width: 0.1,
    jag: 0.16,
    secs: 0.3,
    brightness: 1.6,
};
const BOLT_ARC: Style = Style {
    width: 0.2,
    jag: 0.1,
    secs: 0.32,
    brightness: 2.2,
};
const BOLT_FORK: Style = Style {
    width: 0.06,
    jag: 0.2,
    secs: 0.2,
    brightness: 1.3,
};

#[derive(Clone, Copy)]
enum Effect {
    Arc { from: Vec3, to: Vec3, style: Style },
    Burst { at: Vec3, count: u32 },
    Flash { at: Vec3, strength: f32 },
}

/// Effects waiting to start, with seconds to go: a Spark's strikes and a
/// chain's jumps play out one after another, though the core resolves them
/// at once.
#[derive(Resource, Default)]
struct Queue(Vec<(f32, Effect)>);

/// Turn this frame's spell events into effects.
fn queue_effects(
    mut events: MessageReader<GameEvent>,
    mut queue: ResMut<Queue>,
    cams: Query<&Transform, With<PlayerCamera>>,
    sprites: Query<(&EnemySprite, &GlobalTransform)>,
    ground: Res<GroundHeights>,
    // Where the last strike landed, for a chain's next jump.
    mut last: Local<Option<Vec3>>,
) {
    let Ok(cam) = cams.single() else {
        return;
    };
    let hand = cam.transform_point(HAND);
    let body = |id, at: GridPos| {
        let feet = sprites
            .iter()
            .find(|(s, _)| s.id == id)
            .map(|(_, t)| t.translation())
            .unwrap_or_else(|| {
                let c = cell_center(at);
                c + Vec3::Y * ground.at(c)
            });
        feet + Vec3::Y * CHEST
    };
    for GameEvent(event) in events.read() {
        match *event {
            Event::SpellCast { spell, reach, .. } => {
                *last = Some(hand);
                if spell == Spell::LightningBolt {
                    // To the far side of the last cell: into the wall, or
                    // as far as it carries.
                    let forward = cam.forward().with_y(0.0).normalize_or_zero();
                    let end = cell_center(reach) + forward * 0.5 + Vec3::Y * (hand.y - 0.05);
                    let end = end.with_y(end.y.max(ground.at(end) + 0.2));
                    queue.0.push((
                        0.0,
                        Effect::Arc {
                            from: hand,
                            to: end,
                            style: BOLT_ARC,
                        },
                    ));
                    let mid = hand.lerp(end, 0.5);
                    for k in 0..2 {
                        let off = Vec3::new(0.3 - 0.6 * k as f32, -0.25, 0.0);
                        let fork = mid + cam.rotation * off;
                        queue.0.push((
                            0.02,
                            Effect::Arc {
                                from: mid,
                                to: fork,
                                style: BOLT_FORK,
                            },
                        ));
                    }
                    queue.0.push((
                        0.0,
                        Effect::Flash {
                            at: mid,
                            strength: 1.4,
                        },
                    ));
                }
            }
            Event::SpellHit {
                spell,
                id,
                at,
                order,
            } => {
                let to = body(id, at);
                let k = order as f32;
                match spell {
                    Spell::Spark => {
                        let t = k * SPARK_GAP;
                        queue.0.push((
                            t,
                            Effect::Arc {
                                from: hand,
                                to,
                                style: SPARK_ARC,
                            },
                        ));
                        queue.0.push((t, Effect::Burst { at: to, count: 3 }));
                        queue.0.push((
                            t,
                            Effect::Flash {
                                at: to,
                                strength: 0.35,
                            },
                        ));
                    }
                    Spell::ChainLightning => {
                        let t = k * CHAIN_GAP;
                        let from = last.unwrap_or(hand);
                        queue.0.push((
                            t,
                            Effect::Arc {
                                from,
                                to,
                                style: CHAIN_ARC,
                            },
                        ));
                        queue.0.push((t, Effect::Burst { at: to, count: 4 }));
                        queue.0.push((
                            t,
                            Effect::Flash {
                                at: to,
                                strength: 0.8,
                            },
                        ));
                        *last = Some(to);
                    }
                    Spell::LightningBolt => {
                        queue.0.push((0.03, Effect::Burst { at: to, count: 5 }));
                    }
                }
            }
            _ => {}
        }
    }
}

#[derive(Component)]
struct Arc {
    from: Vec3,
    to: Vec3,
    style: Style,
    age: f32,
    since_jag: f32,
    seed: u64,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

#[derive(Component)]
struct Burst {
    age: f32,
    drift: Vec3,
}

#[derive(Component)]
struct SpellFlash {
    age: f32,
    lumens: f32,
}

/// How long a burst of sparks lasts.
const BURST_SECS: f32 = 0.24;

#[allow(clippy::too_many_arguments)]
fn play_queue(
    mut commands: Commands,
    time: Res<Time>,
    mut queue: ResMut<Queue>,
    assets: Res<SpellAssets>,
    cams: Query<&Transform, With<PlayerCamera>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut seed: Local<u64>,
) {
    let Ok(cam) = cams.single() else {
        return;
    };
    let dt = time.delta_secs();
    let mut due = Vec::new();
    queue.0.retain_mut(|(wait, effect)| {
        *wait -= dt;
        if *wait <= 0.0 {
            due.push(*effect);
            false
        } else {
            true
        }
    });
    for effect in due {
        *seed = seed.wrapping_add(1);
        let roll = |n: u64| zrpg_core::rng::mix64(*seed * 31 + n);
        let unit = |n: u64| (roll(n) % 1000) as f32 / 1000.0 - 0.5;
        match effect {
            Effect::Arc { from, to, style } => {
                let mesh = meshes.add(arc_mesh(from, to, style, cam.translation, *seed));
                let material = materials.add(StandardMaterial {
                    base_color_texture: Some(assets.lightning.clone()),
                    ..glow(brightened(style.brightness))
                });
                commands.spawn((
                    Arc {
                        from,
                        to,
                        style,
                        age: 0.0,
                        since_jag: 0.0,
                        seed: *seed,
                        mesh: mesh.clone(),
                        material: material.clone(),
                    },
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                    Transform::default(),
                    NotShadowCaster,
                ));
            }
            Effect::Burst { at, count } => {
                for n in 0..count as u64 {
                    let jitter = Vec3::new(unit(n * 3), unit(n * 3 + 1), unit(n * 3 + 2)) * 0.3;
                    commands.spawn((
                        Burst {
                            age: -0.03 * n as f32,
                            drift: jitter * 2.0 + Vec3::Y * 0.3,
                        },
                        Mesh3d(assets.spark_mesh.clone()),
                        MeshMaterial3d(assets.sparks[0].clone()),
                        Transform::from_translation(at + jitter).with_rotation(cam.rotation),
                        Visibility::Hidden,
                        NotShadowCaster,
                    ));
                }
            }
            Effect::Flash { at, strength } => {
                commands.spawn((
                    SpellFlash {
                        age: 0.0,
                        lumens: FLASH_LUMENS * strength,
                    },
                    PointLight {
                        color: SPELL_LIGHT,
                        intensity: FLASH_LUMENS * strength,
                        range: FLASH_RANGE,
                        radius: 0.05,
                        shadow_maps_enabled: false,
                        ..default()
                    },
                    Transform::from_translation(at),
                ));
            }
        }
    }
}

/// The spell light at strength `k`: fading out below 1, white-hot above.
fn brightened(k: f32) -> Color {
    let c = SPELL_LIGHT.to_linear();
    let hot = (k - 1.0).max(0.0);
    Color::LinearRgba(LinearRgba::new(
        c.red + hot,
        c.green + hot,
        c.blue + hot,
        k.min(1.0),
    ))
}

/// A jagged ribbon of light from `from` to `to`, turned to face `eye`.
fn arc_mesh(from: Vec3, to: Vec3, style: Style, eye: Vec3, seed: u64) -> Mesh {
    let length = from.distance(to).max(0.01);
    let segments = ((length * 7.0) as usize).clamp(3, 48);
    let dir = (to - from) / length;
    // Two directions across the line to wander in.
    let side = dir.any_orthonormal_vector();
    let up = dir.cross(side);
    let mut points: Vec<Vec3> = (0..=segments)
        .map(|i| {
            let t = i as f32 / segments as f32;
            let h = zrpg_core::rng::mix64(seed.wrapping_mul(977) + i as u64);
            let a = (h % 1000) as f32 / 1000.0 - 0.5;
            let b = (h / 1000 % 1000) as f32 / 1000.0 - 0.5;
            // Pinned at both ends, wildest in the middle.
            let sway = style.jag * length.min(2.0) * (std::f32::consts::PI * t).sin();
            from.lerp(to, t) + (side * a + up * b) * sway
        })
        .collect();
    points[0] = from;
    points[segments] = to;

    let mut positions = Vec::with_capacity(points.len() * 2);
    let mut uvs = Vec::with_capacity(points.len() * 2);
    let mut along = 0.0;
    for (i, &p) in points.iter().enumerate() {
        let next = points[(i + 1).min(segments)];
        let prev = points[i.saturating_sub(1)];
        let tangent = (next - prev).normalize_or_zero();
        if i > 0 {
            along += p.distance(prev);
        }
        // Thin near its start, which may be right by the eye.
        let taper = 0.15 + 0.85 * (along / TAPER).min(1.0);
        let across = tangent.cross(eye - p).normalize_or_zero() * style.width * 0.5 * taper;
        // The strip is 4:1, so it repeats every four widths.
        let u = along / (style.width * 4.0);
        positions.push((p - across).to_array());
        positions.push((p + across).to_array());
        uvs.push([u, 1.0]);
        uvs.push([u, 0.0]);
    }
    let indices: Vec<u32> = (0..segments as u32)
        .flat_map(|i| {
            let k = i * 2;
            [k, k + 1, k + 2, k + 1, k + 3, k + 2]
        })
        .collect();
    let count = positions.len();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; count])
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}

/// Arcs flicker into new shapes and fade out.
fn animate_arcs(
    mut commands: Commands,
    time: Res<Time>,
    cams: Query<&Transform, With<PlayerCamera>>,
    mut arcs: Query<(Entity, &mut Arc)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(cam) = cams.single() else {
        return;
    };
    let dt = time.delta_secs();
    for (entity, mut arc) in &mut arcs {
        arc.age += dt;
        arc.since_jag += dt;
        let left = 1.0 - arc.age / arc.style.secs;
        if left <= 0.0 {
            meshes.remove(&arc.mesh);
            materials.remove(&arc.material);
            commands.entity(entity).despawn();
            continue;
        }
        if arc.since_jag >= JITTER_SECS {
            arc.since_jag = 0.0;
            arc.seed = arc.seed.wrapping_add(7919);
            let mesh = arc_mesh(arc.from, arc.to, arc.style, cam.translation, arc.seed);
            if let Some(mut m) = meshes.get_mut(&arc.mesh) {
                *m = mesh;
            }
        }
        if let Some(mut m) = materials.get_mut(&arc.material) {
            // Bright at once, then a quick fade with a last flicker.
            let flicker = if (arc.age / JITTER_SECS) as u32 % 3 == 2 {
                0.6
            } else {
                1.0
            };
            m.base_color = brightened(arc.style.brightness * left * flicker);
        }
    }
}

/// Spark bursts play their four frames, drifting apart.
fn animate_bursts(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<SpellAssets>,
    mut bursts: Query<(
        Entity,
        &mut Burst,
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut Visibility,
    )>,
) {
    let dt = time.delta_secs();
    for (entity, mut burst, mut t, mut material, mut visible) in &mut bursts {
        burst.age += dt;
        if burst.age < 0.0 {
            continue;
        }
        let frame = (burst.age / BURST_SECS * SPARK_FRAMES as f32) as usize;
        let Some(frame_material) = assets.sparks.get(frame) else {
            commands.entity(entity).despawn();
            continue;
        };
        *visible = Visibility::Inherited;
        if material.0 != *frame_material {
            material.0 = frame_material.clone();
        }
        t.translation += burst.drift * dt;
    }
}

fn fade_flashes(
    mut commands: Commands,
    time: Res<Time>,
    mut flashes: Query<(Entity, &mut SpellFlash, &mut PointLight)>,
) {
    for (entity, mut flash, mut light) in &mut flashes {
        flash.age += time.delta_secs();
        let left = 1.0 - flash.age / FLASH_SECS;
        if left <= 0.0 {
            commands.entity(entity).despawn();
        } else {
            light.intensity = flash.lumens * left * left;
        }
    }
}

/// A spell bar slot, for the spell at this index of `Spell::ALL`.
#[derive(Component)]
struct SpellSlot(usize);
/// The shade over a slot that shrinks as its cooldown runs out.
#[derive(Component)]
struct CooldownShade(usize);

fn spawn_bar(
    mut commands: Commands,
    cams: Query<Entity, With<PlayerCamera>>,
    assets: Res<SpellAssets>,
) {
    let Ok(cam) = cams.single() else {
        return;
    };
    commands
        .spawn((
            UiTargetCamera(cam),
            Node {
                position_type: PositionType::Absolute,
                right: px(6),
                bottom: px(6),
                column_gap: px(3),
                ..default()
            },
        ))
        .with_children(|bar| {
            for (i, icon) in assets.icons.iter().enumerate() {
                bar.spawn((
                    SpellSlot(i),
                    Node {
                        width: px(ICON + 4.0),
                        height: px(ICON + 4.0),
                        padding: UiRect::all(px(1)),
                        border: UiRect::all(px(1)),
                        ..default()
                    },
                    BorderColor::all(Color::srgba(0.6, 0.85, 1.0, 0.45)),
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
                ))
                .with_children(|slot| {
                    slot.spawn(ImageNode::new(icon.clone()));
                    slot.spawn((
                        CooldownShade(i),
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
                    slot.spawn((
                        Text::new(KEYS[i]),
                        TextFont {
                            font_size: FontSize::Px(6.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.75, 0.85, 0.95)),
                        TextShadow {
                            offset: Vec2::new(1.0, 1.0),
                            color: Color::srgba(0.0, 0.0, 0.0, 0.85),
                        },
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(0),
                            top: px(0),
                            ..default()
                        },
                    ));
                });
            }
        });
}

/// Dim a spell you can't afford; shade it while it cools down.
fn update_bar(
    run: Res<Run>,
    slots: Query<&SpellSlot>,
    mut shades: Query<(&CooldownShade, &mut Node)>,
    mut icons: Query<(&ChildOf, &mut ImageNode)>,
) {
    let p = &run.player;
    for (shade, mut node) in &mut shades {
        let spell = Spell::ALL[shade.0];
        let left = p.spell_cooldowns[shade.0] / spell.cooldown();
        let h = px(ICON * left.clamp(0.0, 1.0));
        if node.height != h {
            node.height = h;
        }
    }
    for (parent, mut image) in &mut icons {
        let Ok(slot) = slots.get(parent.parent()) else {
            continue;
        };
        let affordable = p.energy >= Spell::ALL[slot.0].cost();
        let tint = if affordable {
            Color::WHITE
        } else {
            Color::srgba(0.45, 0.45, 0.55, 0.8)
        };
        if image.color != tint {
            image.color = tint;
        }
    }
}
