//! Sound. Every effect is synthesized at startup (see `synth.rs`) and played
//! in response to game events. Zombie sounds and torch crackle come from
//! where they happen, panned and fading with distance, so you can hear which
//! way a zombie is coming and when it is about to strike. A quiet room tone
//! loops underneath everything. `M` mutes.

use std::time::Duration;

use bevy::audio::{
    AudioSinkPlayback, DefaultSpatialScale, PlaybackMode, SpatialAudioSink, SpatialListener,
    SpatialScale, Volume,
};
use bevy::prelude::*;
use zrpg_core::{EnemyKind, Event, GridPos, Tile};

use crate::input::{Action, Actions};
use crate::level::cell_center;
use crate::lighting::Flame;
use crate::synth;
use crate::view::PlayerCamera;
use crate::{Frame, GameEvent, Run};

/// The mixer: each sound's loudness, in one place (linear gain).
mod mix {
    pub const STEP: f32 = 0.35;
    pub const TURN: f32 = 0.1;
    pub const BUMP: f32 = 0.4;
    pub const SWING: f32 = 0.4;
    pub const HIT: f32 = 0.75;
    pub const HURT: f32 = 0.8;
    pub const WHIFF: f32 = 0.45;
    pub const GROAN: f32 = 0.9;
    pub const NOTICE: f32 = 0.6;
    pub const SHUFFLE: f32 = 0.4;
    pub const CRUMBLE: f32 = 0.8;
    pub const DEATH: f32 = 0.8;
    pub const BREATH: f32 = 0.35;
    pub const STAIRS: f32 = 0.5;
    pub const AMBIENCE: f32 = 0.25;
    pub const CRACKLE: f32 = 0.3;
    pub const LEVEL_UP: f32 = 0.6;
    pub const PERK: f32 = 0.5;
}

/// Rodio fades spatial sounds with the inverse square of distance, which is
/// steep at one world unit per cell; this keeps a zombie three cells away
/// clearly audible.
const SPATIAL_SCALE: f32 = 0.45;
/// Distance between the listener's ears, in world units.
const EAR_GAP: f32 = 0.3;
/// Sounds come from about head height.
const SOUND_HEIGHT: f32 = 0.5;
/// How many footstep variations to cycle through.
const STEP_VARIANTS: usize = 3;

pub struct SoundPlugin;

impl Plugin for SoundPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DefaultSpatialScale(SpatialScale::new(SPATIAL_SCALE)))
            .init_resource::<Muted>()
            .add_systems(Startup, synthesize)
            .add_systems(PostStartup, (listen, start_ambience))
            .add_systems(
                Update,
                (toggle_mute, play_event_sounds, crackle_torches).in_set(Frame::Present),
            );
    }
}

#[derive(Resource, Default)]
pub struct Muted(pub bool);

#[derive(Resource)]
struct Sounds {
    steps: Vec<Handle<AudioSource>>,
    turn: Handle<AudioSource>,
    bump: Handle<AudioSource>,
    swing: Handle<AudioSource>,
    whiff: Handle<AudioSource>,
    hit: Handle<AudioSource>,
    hurt: Handle<AudioSource>,
    groan: Handle<AudioSource>,
    notice: Handle<AudioSource>,
    shuffle: Handle<AudioSource>,
    crumble: Handle<AudioSource>,
    death: Handle<AudioSource>,
    breath: Handle<AudioSource>,
    stairs: Handle<AudioSource>,
    ambience: Handle<AudioSource>,
    crackle: Handle<AudioSource>,
    level_up: Handle<AudioSource>,
    perk: Handle<AudioSource>,
}

fn synthesize(mut commands: Commands, mut sources: ResMut<Assets<AudioSource>>) {
    let mut add = |samples: Vec<f32>| {
        sources.add(AudioSource {
            bytes: synth::wav(&samples).into(),
        })
    };
    commands.insert_resource(Sounds {
        steps: (0..STEP_VARIANTS as u64)
            .map(|i| add(synth::footstep(100 + i, 1.0)))
            .collect(),
        turn: add(synth::turn(2)),
        bump: add(synth::bump(3)),
        swing: add(synth::swing(4)),
        whiff: add(synth::whiff(5)),
        hit: add(synth::hit(6)),
        hurt: add(synth::hurt(7)),
        groan: add(synth::groan(8)),
        notice: add(synth::notice(9)),
        shuffle: add(synth::shuffle(10)),
        crumble: add(synth::crumble(11)),
        death: add(synth::death(12)),
        breath: add(synth::breath(13)),
        stairs: add(synth::stairs(14)),
        ambience: add(synth::ambience(15)),
        crackle: add(synth::crackle(16)),
        level_up: add(synth::level_up(17)),
        perk: add(synth::perk(18)),
    });
}

/// The player's ears ride on the camera, so sounds pan as you turn.
fn listen(mut commands: Commands, cams: Query<Entity, With<PlayerCamera>>) {
    for cam in &cams {
        commands.entity(cam).insert(SpatialListener::new(EAR_GAP));
    }
}

fn start_ambience(mut commands: Commands, sounds: Res<Sounds>) {
    commands.spawn((
        AudioPlayer::new(sounds.ambience.clone()),
        PlaybackSettings::LOOP.with_volume(Volume::Linear(mix::AMBIENCE)),
    ));
}

/// Fire-and-forget playback, optionally from a place in the world.
fn play(
    commands: &mut Commands,
    sound: &Handle<AudioSource>,
    gain: f32,
    speed: f32,
    at: Option<Vec3>,
    muted: bool,
) {
    let mut settings = PlaybackSettings::DESPAWN
        .with_volume(Volume::Linear(gain))
        .with_speed(speed);
    settings.muted = muted;
    match at {
        Some(pos) => commands.spawn((
            AudioPlayer::new(sound.clone()),
            settings.with_spatial(true),
            Transform::from_translation(pos),
        )),
        None => commands.spawn((AudioPlayer::new(sound.clone()), settings)),
    };
}

fn ear_level(cell: GridPos) -> Vec3 {
    cell_center(cell) + Vec3::Y * SOUND_HEIGHT
}

/// Each creature's voice: the same synthesized groans, pitched to suit it.
fn voice(kind: EnemyKind) -> f32 {
    match kind {
        EnemyKind::Zombie => 1.0,
        EnemyKind::Undead => 0.92,
        EnemyKind::SkeletonWarrior => 1.25,
        EnemyKind::Mummy => 0.8,
        EnemyKind::RoyalMummy => 0.74,
        EnemyKind::EmberSkeleton => 1.15,
        EnemyKind::Wraith => 1.45,
        EnemyKind::SkeletonKnight => 1.05,
        EnemyKind::InnerDemon => 0.85,
        EnemyKind::DemonKnight => 0.7,
    }
}

/// A little pitch variety so repeated sounds don't machine-gun.
fn wobble(n: u32) -> f32 {
    let h = zrpg_core::rng::mix64(n as u64);
    0.94 + (h % 1000) as f32 / 1000.0 * 0.12
}

fn play_event_sounds(
    mut commands: Commands,
    mut events: MessageReader<GameEvent>,
    run: Res<Run>,
    sounds: Res<Sounds>,
    muted: Res<Muted>,
    mut count: Local<u32>,
) {
    let muted = muted.0;
    let enemy_at = |id| {
        run.enemies
            .iter()
            .find(|e| e.id == id)
            .map(|e| ear_level(e.pos))
    };
    let voice_of = |id| {
        run.enemies
            .iter()
            .find(|e| e.id == id)
            .map_or(1.0, |e| voice(e.kind))
    };
    for GameEvent(event) in events.read() {
        *count = count.wrapping_add(1);
        // A creature's own cries are pitched to its voice.
        let pitch = match *event {
            Event::EnemyNoticed { id }
            | Event::EnemyWindUp { id }
            | Event::EnemyHurt { id, .. } => voice_of(id),
            _ => 1.0,
        };
        let speed = wobble(*count) * pitch;
        let mut sfx = |sound: &Handle<AudioSource>, gain: f32, at: Option<Vec3>| {
            play(&mut commands, sound, gain, speed, at, muted)
        };
        match *event {
            Event::Moved { to, .. } if run.floor.map.get(to) == Tile::StairsDown => {
                sfx(&sounds.stairs, mix::STAIRS, None)
            }
            Event::Moved { .. } => {
                let step = &sounds.steps[*count as usize % sounds.steps.len()];
                sfx(step, mix::STEP, None)
            }
            Event::Turned { .. } => sfx(&sounds.turn, mix::TURN, None),
            Event::Bumped { .. } => sfx(&sounds.bump, mix::BUMP, None),
            Event::Swung { .. } => sfx(&sounds.swing, mix::SWING, None),
            Event::Exhausted => sfx(&sounds.breath, mix::BREATH, None),
            Event::EnemyNoticed { id } => sfx(&sounds.notice, mix::NOTICE, enemy_at(id)),
            Event::EnemyMoved { to, .. } => sfx(&sounds.shuffle, mix::SHUFFLE, Some(ear_level(to))),
            Event::EnemyWindUp { id } => sfx(&sounds.groan, mix::GROAN, enemy_at(id)),
            Event::EnemyStruck { damage, .. } if damage > 0 => sfx(&sounds.hurt, mix::HURT, None),
            Event::EnemyStruck { id, .. } => sfx(&sounds.whiff, mix::WHIFF, enemy_at(id)),
            Event::EnemyHurt { id, .. } => sfx(&sounds.hit, mix::HIT, enemy_at(id)),
            Event::EnemyDied { at, .. } => sfx(&sounds.crumble, mix::CRUMBLE, Some(ear_level(at))),
            Event::PlayerDied => sfx(&sounds.death, mix::DEATH, None),
            Event::Descended { .. } | Event::GainedXp { .. } => {}
            Event::LeveledUp { .. } => sfx(&sounds.level_up, mix::LEVEL_UP, None),
            Event::PerkTaken { .. } => sfx(&sounds.perk, mix::PERK, None),
            Event::Dodged { id } => sfx(&sounds.whiff, mix::WHIFF, enemy_at(id)),
        }
    }
}

/// Every new wall torch gets its own crackle, started at a different point in
/// the loop so neighbouring torches don't crackle in unison.
fn crackle_torches(
    mut commands: Commands,
    flames: Query<Entity, Added<Flame>>,
    sounds: Res<Sounds>,
    muted: Res<Muted>,
    mut count: Local<u32>,
) {
    for flame in &flames {
        *count = count.wrapping_add(1);
        let offset = (zrpg_core::rng::mix64(*count as u64) % 2000) as f32 / 1000.0;
        let mut settings = PlaybackSettings {
            mode: PlaybackMode::Loop,
            volume: Volume::Linear(mix::CRACKLE),
            start_position: Some(Duration::from_secs_f32(offset)),
            ..PlaybackSettings::LOOP
        }
        .with_spatial(true);
        settings.muted = muted.0;
        commands
            .entity(flame)
            .insert((AudioPlayer::new(sounds.crackle.clone()), settings));
    }
}

fn toggle_mute(
    actions: Res<Actions>,
    mut muted: ResMut<Muted>,
    mut sinks: Query<&mut AudioSink>,
    mut spatial: Query<&mut SpatialAudioSink>,
) {
    if !actions.just_pressed(Action::Mute) {
        return;
    }
    muted.0 = !muted.0;
    info!("sound {}", if muted.0 { "muted" } else { "on" });
    for mut sink in &mut sinks {
        if muted.0 { sink.mute() } else { sink.unmute() }
    }
    for mut sink in &mut spatial {
        if muted.0 { sink.mute() } else { sink.unmute() }
    }
}

#[cfg(test)]
mod tests {
    use bevy::audio::Decodable;

    use super::*;

    /// The synthesized WAVs must decode with the same decoder the game uses.
    #[test]
    fn synthesized_sounds_decode() {
        for (name, samples) in synth::all() {
            let source = AudioSource {
                bytes: synth::wav(&samples).into(),
            };
            let decoded = source.decoder().count();
            assert_eq!(
                decoded,
                samples.len(),
                "{name} decoded to {decoded} samples"
            );
        }
    }
}
