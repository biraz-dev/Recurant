//! A tiny sound synthesizer. Every sound in the game is generated here from
//! oscillators, noise and filters, then wrapped as an in-memory WAV file, so
//! no audio files ship with the game.
//!
//! Plain Rust with no engine types: each recipe is a function returning mono
//! samples in `-1.0..=1.0` at [`RATE`]. `cargo run -p zrpg --example sfx`
//! writes them all to `target/sfx/` for auditioning.

use std::f32::consts::{PI, TAU};

/// Sample rate. Low, like the picture; plenty for these sounds.
pub const RATE: u32 = 22_050;

fn samples(secs: f32) -> usize {
    (secs * RATE as f32) as usize
}

fn time(i: usize) -> f32 {
    i as f32 / RATE as f32
}

/// Deterministic white noise (xorshift64*), so sounds are reproducible.
struct Noise(u64);

impl Noise {
    fn new(seed: u64) -> Self {
        Noise(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn next(&mut self) -> f32 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        let x = self.0.wrapping_mul(0x2545_F491_4F6C_DD1D);
        (x >> 40) as f32 / (1u64 << 23) as f32 - 1.0
    }

    /// Uniform in `0..1`.
    fn unit(&mut self) -> f32 {
        (self.next() + 1.0) * 0.5
    }
}

/// One-pole low-pass filter.
struct LowPass {
    a: f32,
    y: f32,
}

impl LowPass {
    fn new(cutoff: f32) -> Self {
        LowPass {
            a: 1.0 - (-TAU * cutoff / RATE as f32).exp(),
            y: 0.0,
        }
    }

    fn run(&mut self, x: f32) -> f32 {
        self.y += self.a * (x - self.y);
        self.y
    }
}

/// State-variable band-pass whose centre frequency can move every sample.
struct BandPass {
    low: f32,
    band: f32,
    damping: f32,
}

impl BandPass {
    /// `damping`: about 0.2 (narrow, ringing) to 1.5 (wide, breathy).
    fn new(damping: f32) -> Self {
        BandPass {
            low: 0.0,
            band: 0.0,
            damping,
        }
    }

    fn run(&mut self, x: f32, centre: f32) -> f32 {
        let f = 2.0 * (PI * centre.min(RATE as f32 / 6.0) / RATE as f32).sin();
        self.low += f * self.band;
        let high = x - self.low - self.damping * self.band;
        self.band += f * high;
        self.band
    }
}

/// A sine oscillator with a glide-friendly phase accumulator.
#[derive(Default)]
struct Osc {
    phase: f32,
}

impl Osc {
    fn sine(&mut self, freq: f32) -> f32 {
        self.phase = (self.phase + freq / RATE as f32).fract();
        (self.phase * TAU).sin()
    }

    fn saw(&mut self, freq: f32) -> f32 {
        self.phase = (self.phase + freq / RATE as f32).fract();
        self.phase * 2.0 - 1.0
    }
}

/// Exponential decay with time constant `tau` seconds.
fn decay(t: f32, tau: f32) -> f32 {
    (-t / tau).exp()
}

/// A smooth bump over `0..len` seconds (zero at both ends).
fn bell(t: f32, len: f32) -> f32 {
    if (0.0..len).contains(&t) {
        (PI * t / len).sin().powi(2)
    } else {
        0.0
    }
}

/// Mix `src` into `dst` starting at `at` seconds, scaled by `gain`.
fn mix_in(dst: &mut [f32], src: &[f32], at: f32, gain: f32) {
    let start = samples(at);
    for (d, s) in dst.iter_mut().skip(start).zip(src) {
        *d += s * gain;
    }
}

/// Feedback echo, for stone rooms.
fn echo(buf: &mut [f32], delay: f32, feedback: f32) {
    let d = samples(delay).max(1);
    for i in d..buf.len() {
        buf[i] += buf[i - d] * feedback;
    }
}

/// Scale to a common peak. Each sound's loudness in the game is then set in
/// one place, the mixer.
fn normalize(mut buf: Vec<f32>) -> Vec<f32> {
    let peak = buf.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    if peak > 0.0 {
        for s in &mut buf {
            *s *= 0.9 / peak;
        }
    }
    buf
}

/// Fade a one-shot's very ends, so it never clicks, then normalize it.
fn finish(mut buf: Vec<f32>) -> Vec<f32> {
    let fade = samples(0.003).min(buf.len() / 2);
    let len = buf.len();
    for i in 0..fade {
        let k = i as f32 / fade as f32;
        buf[i] *= k;
        buf[len - 1 - i] *= k;
    }
    normalize(buf)
}

/// Make a loop seamless by cross-fading its tail into its head.
fn seamless(mut buf: Vec<f32>, overlap: f32) -> Vec<f32> {
    let n = samples(overlap).min(buf.len() / 2);
    let len = buf.len() - n;
    for i in 0..n {
        let k = i as f32 / n as f32;
        buf[i] = buf[i] * k + buf[len + i] * (1.0 - k);
    }
    buf.truncate(len);
    buf
}

// --- the sounds ----------------------------------------------------------------

/// A footstep on stone: a soft thump, the body of the foot, a little grit.
/// `pitch` scales everything (1.0 = normal).
pub fn footstep(seed: u64, pitch: f32) -> Vec<f32> {
    let mut noise = Noise::new(seed);
    let (mut body, mut grit_lp) = (LowPass::new(700.0 * pitch), LowPass::new(2500.0));
    let mut osc = Osc::default();
    let buf = (0..samples(0.14))
        .map(|i| {
            let t = time(i);
            let n = noise.next();
            let thump = osc.sine(62.0 * pitch * (1.0 + 0.6 * decay(t, 0.01))) * decay(t, 0.028);
            let grit = (n - grit_lp.run(n)) * decay(t, 0.045);
            0.9 * thump + 0.8 * body.run(n) * decay(t, 0.02) + 0.15 * grit
        })
        .collect();
    finish(buf)
}

/// Turning on the spot: a quiet swish of cloth.
pub fn turn(seed: u64) -> Vec<f32> {
    let mut noise = Noise::new(seed);
    let mut bp = BandPass::new(1.2);
    let len = 0.16;
    finish(
        (0..samples(len))
            .map(|i| bp.run(noise.next(), 1400.0) * bell(time(i), len))
            .collect(),
    )
}

/// Walking into a wall: a dull thud.
pub fn bump(seed: u64) -> Vec<f32> {
    let mut noise = Noise::new(seed);
    let mut lp = LowPass::new(300.0);
    let mut osc = Osc::default();
    finish(
        (0..samples(0.16))
            .map(|i| {
                let t = time(i);
                osc.sine(50.0) * decay(t, 0.05) + 0.8 * lp.run(noise.next()) * decay(t, 0.03)
            })
            .collect(),
    )
}

/// A blade cutting air: noise through a band-pass sweeping up and back.
fn whoosh(seed: u64, len: f32, low: f32, high: f32) -> Vec<f32> {
    let mut noise = Noise::new(seed);
    let mut bp = BandPass::new(0.9);
    (0..samples(len))
        .map(|i| {
            let t = time(i);
            let sweep = (PI * t / len).sin().powf(1.5);
            bp.run(noise.next(), low + (high - low) * sweep) * bell(t, len)
        })
        .collect()
}

/// The player's swing.
pub fn swing(seed: u64) -> Vec<f32> {
    finish(whoosh(seed, 0.2, 500.0, 2700.0))
}

/// A zombie's blow that misses: lower and heavier than the player's swing.
pub fn whiff(seed: u64) -> Vec<f32> {
    finish(whoosh(seed, 0.26, 300.0, 1100.0))
}

/// The sword landing on bone and rags: a thump, a crunch and a crack.
pub fn hit(seed: u64) -> Vec<f32> {
    let mut noise = Noise::new(seed);
    let (mut crunch, mut slap) = (BandPass::new(1.0), BandPass::new(0.7));
    let mut osc = Osc::default();
    let mut buf: Vec<f32> = (0..samples(0.24))
        .map(|i| {
            let t = time(i);
            let n = noise.next();
            let thump = osc.sine(60.0 + 80.0 * decay(t, 0.015)) * decay(t, 0.035);
            0.9 * thump
                + 0.8 * slap.run(n, 1800.0) * decay(t, 0.012)
                + 0.6 * crunch.run(n, 900.0) * decay(t, 0.05)
        })
        .collect();
    // Bone cracks: tiny bright ringing clicks.
    for (k, (at, freq)) in [(0.0, 2600.0), (0.008, 3100.0), (0.017, 2300.0)]
        .into_iter()
        .enumerate()
    {
        let mut osc = Osc::default();
        let click: Vec<f32> = (0..samples(0.02))
            .map(|i| osc.sine(freq) * decay(time(i), 0.0025))
            .collect();
        mix_in(&mut buf, &click, at, 0.5 - k as f32 * 0.1);
    }
    finish(buf)
}

/// A zombie's blow landing on the player: heavier, lower, closer.
pub fn hurt(seed: u64) -> Vec<f32> {
    let mut noise = Noise::new(seed);
    let (mut lp, mut crunch) = (LowPass::new(500.0), BandPass::new(1.0));
    let mut osc = Osc::default();
    finish(
        (0..samples(0.35))
            .map(|i| {
                let t = time(i);
                let n = noise.next();
                osc.sine(42.0 + 53.0 * decay(t, 0.02)) * decay(t, 0.07)
                    + 0.9 * lp.run(n) * decay(t, 0.06)
                    + 0.5 * crunch.run(n, 1200.0) * decay(t, 0.025)
            })
            .collect(),
    )
}

/// The zombie's tell before it strikes: a rasping groan that falls in pitch.
/// It lasts about as long as the wind-up, so you can react by ear.
pub fn groan(seed: u64) -> Vec<f32> {
    voice(seed, 0.55, 92.0, 0.8)
}

/// A dry, croaking voice: a sawtooth through two vowel-like resonances, with
/// wobble, breath and a little distortion.
fn voice(seed: u64, len: f32, base: f32, fall: f32) -> Vec<f32> {
    let mut noise = Noise::new(seed);
    let mut jitter = LowPass::new(12.0);
    let (mut f1, mut f2) = (BandPass::new(0.35), BandPass::new(0.45));
    let mut breath = LowPass::new(1500.0);
    let mut osc = Osc::default();
    let buf = (0..samples(len))
        .map(|i| {
            let t = time(i);
            let n = noise.next();
            let pitch = base * (1.0 - (1.0 - fall) * t / len)
                + 2.5 * (TAU * 5.5 * t).sin()
                + 30.0 * jitter.run(n);
            let saw = osc.saw(pitch.max(20.0));
            let raw = f1.run(saw, 520.0) + 0.6 * f2.run(saw, 1150.0) + 0.25 * breath.run(n);
            let env = (t / 0.06).min(1.0) * ((len - t) / 0.12).clamp(0.0, 1.0);
            let wobble = 1.0 + 0.15 * (TAU * 7.0 * t).sin();
            (raw * 2.5).tanh() * env * wobble
        })
        .collect();
    finish(buf)
}

/// A zombie noticing you: a hiss with a croak behind it.
pub fn notice(seed: u64) -> Vec<f32> {
    let mut noise = Noise::new(seed);
    let mut bp = BandPass::new(0.6);
    let len = 0.5;
    let mut buf: Vec<f32> = (0..samples(len))
        .map(|i| {
            let t = time(i);
            let env = (t / 0.03).min(1.0) * decay(t, 0.18);
            bp.run(noise.next(), 2800.0) * env * (1.0 + 0.3 * (TAU * 23.0 * t).sin())
        })
        .collect();
    let croak = voice(seed + 1, 0.3, 75.0, 0.85);
    mix_in(&mut buf, &croak, 0.12, 0.5);
    finish(buf)
}

/// A zombie shuffling a step: a scrape, then the foot coming down.
pub fn shuffle(seed: u64) -> Vec<f32> {
    let mut noise = Noise::new(seed);
    let mut bp = BandPass::new(0.8);
    let mut osc = Osc::default();
    let len = 0.3;
    finish(
        (0..samples(len))
            .map(|i| {
                let t = time(i);
                let drag = bell(t, 0.24) * (0.6 + 0.4 * (TAU * 18.0 * t).sin());
                let plant = if t > 0.2 {
                    osc.sine(70.0) * decay(t - 0.2, 0.025)
                } else {
                    0.0
                };
                bp.run(noise.next(), 650.0) * drag + 0.7 * plant
            })
            .collect(),
    )
}

/// A zombie falling apart: a thud, a rumble and a rattle of bones.
pub fn crumble(seed: u64) -> Vec<f32> {
    let mut noise = Noise::new(seed);
    let (mut lp, mut rumble) = (LowPass::new(400.0), LowPass::new(250.0));
    let mut osc = Osc::default();
    let len = 0.75;
    let mut buf: Vec<f32> = (0..samples(len))
        .map(|i| {
            let t = time(i);
            let n = noise.next();
            osc.sine(45.0 + 30.0 * decay(t, 0.02)) * decay(t, 0.06)
                + 0.7 * lp.run(n) * decay(t, 0.08)
                + 0.5 * rumble.run(n) * decay(t, 0.25)
        })
        .collect();
    for _ in 0..14 {
        let u = noise.unit();
        let at = 0.03 + 0.6 * u.powf(1.6);
        let freq = 1400.0 + 2200.0 * noise.unit();
        let tau = 0.003 + 0.003 * noise.unit();
        let mut osc = Osc::default();
        let click: Vec<f32> = (0..samples(0.03))
            .map(|i| osc.sine(freq) * decay(time(i), tau))
            .collect();
        mix_in(
            &mut buf,
            &click,
            at,
            (0.3 + 0.5 * noise.unit()) * decay(at, 0.4),
        );
    }
    finish(buf)
}

/// The player dying: a low boom and a sinking minor third, echoing away.
pub fn death(seed: u64) -> Vec<f32> {
    let mut noise = Noise::new(seed);
    let mut lp = LowPass::new(180.0);
    let (mut boom, mut a, mut b) = (Osc::default(), Osc::default(), Osc::default());
    let len = 1.6;
    let mut buf: Vec<f32> = (0..samples(len))
        .map(|i| {
            let t = time(i);
            let sink = 1.0 - 0.18 * t / len;
            let drone = (a.sine(196.0 * sink) + b.sine(233.1 * sink))
                * (t / 0.2).min(1.0)
                * ((len - t) / 0.6).clamp(0.0, 1.0);
            boom.sine(48.0) * decay(t, 0.35)
                + 0.6 * lp.run(noise.next()) * decay(t, 0.5)
                + 0.25 * drone
        })
        .collect();
    echo(&mut buf, 0.18, 0.4);
    finish(buf)
}

/// Swinging without the energy for it: a short, tired breath.
pub fn breath(seed: u64) -> Vec<f32> {
    let mut noise = Noise::new(seed);
    let (mut lp, mut bp) = (LowPass::new(1800.0), BandPass::new(1.4));
    let len = 0.35;
    finish(
        (0..samples(len))
            .map(|i| bp.run(lp.run(noise.next()), 900.0) * bell(time(i), len))
            .collect(),
    )
}

/// Taking the stairs: footsteps going down and away, in an echoing stairwell.
pub fn stairs(seed: u64) -> Vec<f32> {
    let mut buf = vec![0.0; samples(0.9)];
    for (k, (pitch, gain)) in [(1.0, 1.0), (0.92, 0.8), (0.85, 0.6), (0.78, 0.45)]
        .into_iter()
        .enumerate()
    {
        mix_in(
            &mut buf,
            &footstep(seed + k as u64, pitch),
            0.14 * k as f32,
            gain,
        );
    }
    echo(&mut buf, 0.11, 0.35);
    finish(buf)
}

/// The dungeon's room tone, looped: a low rumble that swells and settles,
/// and water dripping somewhere.
pub fn ambience(seed: u64) -> Vec<f32> {
    let mut noise = Noise::new(seed);
    let mut lp = LowPass::new(180.0);
    let (len, overlap) = (8.0, 0.5);
    let mut brown = 0.0f32;
    let mut buf: Vec<f32> = (0..samples(len + overlap))
        .map(|i| {
            let t = time(i);
            brown = brown * 0.995 + noise.next() * 0.02;
            let swell = 0.75 + 0.25 * (TAU * t / len).sin();
            lp.run(brown) * swell * 6.0
        })
        .collect();
    for at in [1.3, 3.9, 6.2] {
        let mut osc = Osc::default();
        let drip: Vec<f32> = (0..samples(0.12))
            .map(|i| {
                let t = time(i);
                osc.sine(900.0 + 1400.0 * decay(t, 0.012)) * decay(t, 0.05)
            })
            .collect();
        mix_in(&mut buf, &drip, at, 0.12);
        mix_in(&mut buf, &drip, at + 0.03, 0.04);
    }
    normalize(seamless(buf, overlap))
}

/// A torch burning, looped: sparse pops over a soft flutter.
pub fn crackle(seed: u64) -> Vec<f32> {
    let mut noise = Noise::new(seed);
    let (mut flutter, mut pop_lp) = (LowPass::new(400.0), LowPass::new(3000.0));
    let (len, overlap) = (2.0, 0.2);
    let mut buf: Vec<f32> = (0..samples(len + overlap))
        .map(|i| {
            let t = time(i);
            0.15 * flutter.run(noise.next()) * (1.0 + 0.5 * (TAU * 3.0 * t).sin())
        })
        .collect();
    for _ in 0..40 {
        let at = noise.unit() * (len + overlap - 0.02);
        let (gain, tau) = (0.2 + 0.8 * noise.unit(), 0.001 + 0.002 * noise.unit());
        let pop: Vec<f32> = (0..samples(0.015))
            .map(|i| {
                let n = noise.next();
                (n - pop_lp.run(n)) * decay(time(i), tau)
            })
            .collect();
        mix_in(&mut buf, &pop, at, gain);
    }
    normalize(seamless(buf, overlap))
}

/// Wrap samples as a 16-bit mono PCM WAV file.
/// A struck bell: a few inharmonic partials, each fading at its own pace.
fn chime(freq: f32, len: f32) -> Vec<f32> {
    const PARTIALS: [(f32, f32, f32); 4] = [
        (1.0, 1.0, 0.9),
        (2.0, 0.45, 0.5),
        (2.76, 0.3, 0.3),
        (5.4, 0.12, 0.12),
    ];
    (0..samples(len))
        .map(|i| {
            let t = time(i);
            let attack = (t / 0.004).min(1.0);
            PARTIALS
                .iter()
                .map(|&(ratio, gain, tau)| (TAU * freq * ratio * t).sin() * gain * decay(t, tau))
                .sum::<f32>()
                * attack
        })
        .collect()
}

/// Levelling up: four bells rising, ringing in the stone.
pub fn level_up(_seed: u64) -> Vec<f32> {
    let mut buf = vec![0.0; samples(1.6)];
    for (k, freq) in [392.0, 523.25, 659.25, 783.99].into_iter().enumerate() {
        mix_in(
            &mut buf,
            &chime(freq, 1.2),
            k as f32 * 0.09,
            1.0 - 0.1 * k as f32,
        );
    }
    echo(&mut buf, 0.13, 0.3);
    finish(buf)
}

/// Taking an ability: one low, warm bell.
pub fn perk(_seed: u64) -> Vec<f32> {
    let mut buf = chime(261.63, 0.9);
    let fifth = chime(392.0, 0.7);
    mix_in(&mut buf, &fifth, 0.0, 0.4);
    echo(&mut buf, 0.11, 0.25);
    finish(buf)
}

/// One small coin: a bright, short metallic ping.
fn ping(freq: f32) -> Vec<f32> {
    (0..samples(0.25))
        .map(|i| {
            let t = time(i);
            let attack = (t / 0.002).min(1.0);
            ((TAU * freq * t).sin() + 0.5 * (TAU * freq * 2.71 * t).sin() * decay(t, 0.04))
                * decay(t, 0.07)
                * attack
        })
        .collect()
}

/// Picking up gold: two coins chinking together.
pub fn coins(_seed: u64) -> Vec<f32> {
    let mut buf = vec![0.0; samples(0.4)];
    mix_in(&mut buf, &ping(2637.0), 0.0, 1.0);
    mix_in(&mut buf, &ping(3136.0), 0.07, 0.8);
    finish(buf)
}

/// Gold spilling from a kill: a scatter of coins hitting stone.
pub fn coin_drop(seed: u64) -> Vec<f32> {
    let mut noise = Noise::new(seed);
    let mut buf = vec![0.0; samples(0.6)];
    for k in 0..6 {
        let at = 0.03 + k as f32 * 0.06 + (noise.next() * 0.02).abs();
        let freq = 2200.0 + 900.0 * noise.next().abs();
        mix_in(&mut buf, &ping(freq), at, 1.0 - 0.13 * k as f32);
    }
    finish(buf)
}

pub fn wav(samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2).to_le_bytes()); // bytes per second
    out.extend_from_slice(&2u16.to_le_bytes()); // bytes per frame
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    out
}

/// Every sound by name, for the game and for auditioning.
pub fn all() -> Vec<(&'static str, Vec<f32>)> {
    vec![
        ("step", footstep(1, 1.0)),
        ("turn", turn(2)),
        ("bump", bump(3)),
        ("swing", swing(4)),
        ("whiff", whiff(5)),
        ("hit", hit(6)),
        ("hurt", hurt(7)),
        ("groan", groan(8)),
        ("notice", notice(9)),
        ("shuffle", shuffle(10)),
        ("crumble", crumble(11)),
        ("death", death(12)),
        ("breath", breath(13)),
        ("stairs", stairs(14)),
        ("ambience", ambience(15)),
        ("crackle", crackle(16)),
        ("level_up", level_up(17)),
        ("perk", perk(18)),
        ("coins", coins(19)),
        ("coin_drop", coin_drop(20)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sound_is_clean_and_audible() {
        for (name, s) in all() {
            assert!(s.len() > samples(0.1), "{name} too short");
            assert!(s.iter().all(|x| x.is_finite()), "{name} has NaN/inf");
            let peak = s.iter().fold(0.0f32, |m, x| m.max(x.abs()));
            assert!((0.85..=0.91).contains(&peak), "{name} peak {peak}");
            let rms = (s.iter().map(|x| x * x).sum::<f32>() / s.len() as f32).sqrt();
            assert!(rms > 0.02, "{name} nearly silent (rms {rms})");
            // One-shots fade their ends so they never click; loops are
            // checked for a seamless join instead.
            if !matches!(name, "ambience" | "crackle") {
                assert!(
                    s[0].abs() < 0.01 && s[s.len() - 1].abs() < 0.01,
                    "{name} clicks"
                );
            }
        }
    }

    #[test]
    fn sounds_are_reproducible() {
        assert_eq!(hit(6), hit(6));
        assert_ne!(footstep(1, 1.0), footstep(2, 1.0));
    }

    #[test]
    fn loops_join_without_a_jump() {
        for (name, s) in [("ambience", ambience(15)), ("crackle", crackle(16))] {
            // The seam may only jump as much as the sound does elsewhere.
            let typical = s.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f32>() / s.len() as f32;
            let seam = (s[0] - s[s.len() - 1]).abs();
            assert!(
                seam < typical * 20.0 + 0.02,
                "{name} loop seam {seam} vs {typical}"
            );
        }
    }

    #[test]
    fn wav_header_is_valid() {
        let w = wav(&[0.0, 0.5, -0.5, 1.0]);
        assert_eq!(&w[0..4], b"RIFF");
        assert_eq!(&w[8..16], b"WAVEfmt ");
        assert_eq!(u32::from_le_bytes(w[24..28].try_into().unwrap()), RATE);
        assert_eq!(u32::from_le_bytes(w[40..44].try_into().unwrap()), 8);
        assert_eq!(w.len(), 44 + 8);
        assert_eq!(i16::from_le_bytes([w[50], w[51]]), 32767);
    }
}
