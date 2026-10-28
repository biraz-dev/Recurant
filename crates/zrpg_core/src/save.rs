//! Saving and restoring a run, so auto mode's progress survives closing the
//! game. A save is small: the floor itself is not stored, because the same
//! run seed and depth always generate the same floor; only what changes
//! (the player, the enemies, waiting level-ups, the RNG) is.
//!
//! The format is versioned little-endian bytes. A save that doesn't parse,
//! or comes from an unknown version, is ignored rather than trusted.
//! Version 2 added gold, version 3 six creature items, version 4 kept only
//! the Health Potion, version 5 arrows (in the quiver, lying about and in
//! the air); older saves still load, with what they lack left empty (and a
//! full starting quiver).

use alloc::vec::Vec;

use crate::archery::{Flight, tuning as arrow_tuning};
use crate::dungeon::Floor;
use crate::enemy::{Enemy, EnemyKind, EnemyState};
use crate::game::{Drop, Game, Player};
use crate::grid::{Facing, GridPos};
use crate::items::{Bag, Item};
use crate::progression::{MAX_LEVEL, Perk, Perks};
use crate::rng::Rng;

const MAGIC: &[u8; 4] = b"ZRS5";
const MAGIC_V4: &[u8; 4] = b"ZRS4";
const MAGIC_V3: &[u8; 4] = b"ZRS3";
/// Version 3 carried six kinds of item (none of them potions), a ward and
/// empowered hits; their bytes are read and set aside.
const V3_ITEMS: usize = 6;
const MAGIC_V2: &[u8; 4] = b"ZRS2";
const MAGIC_V1: &[u8; 4] = b"ZRS1";

impl Game {
    /// The run as bytes, for [`Game::load`].
    pub fn save(&self) -> Vec<u8> {
        let mut w = Vec::new();
        w.extend_from_slice(MAGIC);
        put_u64(&mut w, self.run_seed);
        put_u32(&mut w, self.depth);
        w.push(self.params.ruins_entrance as u8);
        w.push(self.params.randomize as u8);
        put_u64(&mut w, self.rng.state());

        let p = &self.player;
        put_i32(&mut w, p.pos.x);
        put_i32(&mut w, p.pos.y);
        w.push(p.facing as u8);
        put_i32(&mut w, p.hp);
        put_i32(&mut w, p.max_hp);
        put_f32(&mut w, p.energy);
        put_f32(&mut w, p.max_energy);
        put_f32(&mut w, p.attack_cooldown);
        put_u32(&mut w, p.level);
        put_u32(&mut w, p.xp);
        put_f32(&mut w, p.regen);
        w.extend(Perk::ALL.map(|perk| p.perks.count(perk)));
        put_u32(&mut w, p.gold);
        w.extend(p.bag.counts());
        put_u32(&mut w, p.arrows);
        put_f32(&mut w, p.bow_cooldown);
        put_u32(&mut w, self.drops.len() as u32);
        for d in &self.drops {
            put_i32(&mut w, d.pos.x);
            put_i32(&mut w, d.pos.y);
            put_u32(&mut w, d.gold);
            w.extend(d.items.counts());
            put_u32(&mut w, d.arrows);
        }
        put_u32(&mut w, self.next_flight);
        put_u32(&mut w, self.flights.len() as u32);
        for f in &self.flights {
            put_u32(&mut w, f.id);
            put_i32(&mut w, f.from.x);
            put_i32(&mut w, f.from.y);
            w.push(f.dir as u8);
            put_f32(&mut w, f.dist);
            put_i32(&mut w, f.cell.x);
            put_i32(&mut w, f.cell.y);
            put_u32(&mut w, f.entered);
        }

        put_u32(&mut w, self.pending.len() as u32);
        for offer in &self.pending {
            w.push(offer.len() as u8);
            w.extend(offer.iter().map(|perk| perk.index() as u8));
        }

        put_u32(&mut w, self.enemies.len() as u32);
        for e in &self.enemies {
            put_u32(&mut w, e.id);
            w.push(kind_index(e.kind) as u8);
            put_i32(&mut w, e.pos.x);
            put_i32(&mut w, e.pos.y);
            put_i32(&mut w, e.hp);
            put_f32(&mut w, e.step_timer);
            let (tag, left) = match e.state {
                EnemyState::Dormant => (0, 0.0),
                EnemyState::Chasing => (1, 0.0),
                EnemyState::WindUp { left } => (2, left),
                EnemyState::Recover { left } => (3, left),
            };
            w.push(tag);
            put_f32(&mut w, left);
        }
        w
    }

    /// A run from [`Game::save`] bytes, or `None` if they aren't a valid
    /// save from this version.
    pub fn load(bytes: &[u8]) -> Option<Game> {
        let mut r = Reader { bytes, at: 0 };
        let version = match r.take(4)? {
            m if m == MAGIC => 5,
            m if m == MAGIC_V4 => 4,
            m if m == MAGIC_V3 => 3,
            m if m == MAGIC_V2 => 2,
            m if m == MAGIC_V1 => 1,
            _ => return None,
        };
        let run_seed = r.u64()?;
        let depth = r.u32()?;
        if depth == 0 {
            return None;
        }
        let mut game = Game::new(run_seed);
        game.params.ruins_entrance = r.u8()? != 0;
        game.params.randomize = r.u8()? != 0;
        game.depth = depth;
        game.floor = Floor::for_depth(run_seed, depth, &game.params);
        game.rng = Rng::new(r.u64()?);

        let map = &game.floor.map;
        let pos = GridPos::new(r.i32()?, r.i32()?);
        let facing = *Facing::ALL.get(r.u8()? as usize)?;
        if !map.is_walkable(pos) {
            return None;
        }
        let mut player = Player {
            pos,
            facing,
            hp: r.i32()?,
            max_hp: r.i32()?,
            energy: r.f32()?,
            max_energy: r.f32()?,
            attack_cooldown: r.f32()?,
            level: r.u32()?.clamp(1, MAX_LEVEL),
            xp: r.u32()?,
            perks: Perks::default(),
            gold: 0,
            bag: Bag::default(),
            // Gear can't change yet, so it isn't saved.
            equipment: crate::Equipment::starting_kit(),
            arrows: arrow_tuning::START_ARROWS,
            bow_cooldown: 0.0,
            spell_cooldowns: [0.0; crate::spells::Spell::ALL.len()],
            regen: r.f32()?,
        };
        let mut counts = [0u8; Perk::ALL.len()];
        for c in counts.iter_mut() {
            *c = r.u8()?;
        }
        player.perks = Perks::from_counts(counts);
        if version >= 2 {
            player.gold = r.u32()?;
            if version >= 4 {
                player.bag = Bag::from_counts(r.bag()?);
                if version >= 5 {
                    player.arrows = r.u32()?.min(arrow_tuning::MAX_ARROWS);
                    player.bow_cooldown = r.f32()?;
                }
            } else if version == 3 {
                r.take(V3_ITEMS + 2)?;
            }
            let mut drops = Vec::new();
            for _ in 0..r.u32()?.min(256) {
                let pos = GridPos::new(r.i32()?, r.i32()?);
                let gold = r.u32()?;
                let items = match version {
                    4.. => Bag::from_counts(r.bag()?),
                    3 => {
                        r.take(V3_ITEMS)?;
                        Bag::default()
                    }
                    _ => Bag::default(),
                };
                let arrows = if version >= 5 { r.u32()? } else { 0 };
                if !game.floor.map.is_walkable(pos) {
                    return None;
                }
                drops.push(Drop {
                    pos,
                    gold,
                    items,
                    arrows,
                });
            }
            game.drops = drops;
            if version >= 5 {
                game.next_flight = r.u32()?;
                let mut flights = Vec::new();
                for _ in 0..r.u32()?.min(64) {
                    let id = r.u32()?;
                    let from = GridPos::new(r.i32()?, r.i32()?);
                    let dir = *Facing::ALL.get(r.u8()? as usize)?;
                    let mut f = Flight::new(id, from, dir);
                    f.dist = r.f32()?;
                    f.cell = GridPos::new(r.i32()?, r.i32()?);
                    f.entered = r.u32()?;
                    flights.push(f);
                }
                game.flights = flights;
            }
        }
        game.player = player;

        let mut pending = Vec::new();
        for _ in 0..r.u32()?.min(MAX_LEVEL) {
            let n = r.u8()?;
            let mut offer = Vec::new();
            for _ in 0..n {
                offer.push(*Perk::ALL.get(r.u8()? as usize)?);
            }
            pending.push(offer);
        }
        game.pending = pending;

        let mut enemies = Vec::new();
        for _ in 0..r.u32()?.min(64) {
            let id = r.u32()?;
            let kind = *EnemyKind::ALL.get(r.u8()? as usize)?;
            let pos = GridPos::new(r.i32()?, r.i32()?);
            let mut e = Enemy::of_kind(id, pos, kind);
            e.hp = r.i32()?;
            e.step_timer = r.f32()?;
            let tag = r.u8()?;
            let left = r.f32()?;
            e.state = match tag {
                0 => EnemyState::Dormant,
                1 => EnemyState::Chasing,
                2 => EnemyState::WindUp { left },
                3 => EnemyState::Recover { left },
                _ => return None,
            };
            if !map.is_walkable(pos) {
                return None;
            }
            enemies.push(e);
        }
        game.enemies = enemies;
        (r.at == bytes.len()).then_some(game)
    }
}

fn kind_index(kind: EnemyKind) -> usize {
    EnemyKind::ALL.iter().position(|&k| k == kind).unwrap_or(0)
}

fn put_u32(w: &mut Vec<u8>, v: u32) {
    w.extend_from_slice(&v.to_le_bytes());
}
fn put_i32(w: &mut Vec<u8>, v: i32) {
    w.extend_from_slice(&v.to_le_bytes());
}
fn put_u64(w: &mut Vec<u8>, v: u64) {
    w.extend_from_slice(&v.to_le_bytes());
}
fn put_f32(w: &mut Vec<u8>, v: f32) {
    w.extend_from_slice(&v.to_le_bytes());
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Option<&[u8]> {
        let s = self.bytes.get(self.at..self.at + n)?;
        self.at += n;
        Some(s)
    }
    fn u8(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }
    fn bag(&mut self) -> Option<[u8; Item::ALL.len()]> {
        self.take(Item::ALL.len())?.try_into().ok()
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn i32(&mut self) -> Option<i32> {
        Some(i32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }
    fn f32(&mut self) -> Option<f32> {
        let v = f32::from_le_bytes(self.take(4)?.try_into().ok()?);
        v.is_finite().then_some(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Command, Event};

    /// Play a while with the bot, banking level-ups.
    fn played(seed: u64, ticks: usize) -> Game {
        let mut g = Game::new(seed);
        let mut out = Vec::new();
        for _ in 0..ticks {
            if let Some(cmd) = crate::auto::next_command(&g) {
                g.apply(cmd, &mut out);
            }
            g.tick(1.0 / 30.0, &mut out);
            if !g.player.is_alive() {
                break;
            }
        }
        g
    }

    #[test]
    fn a_saved_run_comes_back_exactly() {
        for seed in [1u64, 7, 42] {
            let g = played(seed, 9000);
            let bytes = g.save();
            assert!(bytes.len() < 1024, "{} bytes", bytes.len());
            let back = Game::load(&bytes).expect("loads");
            assert_eq!(back.save(), bytes, "seed {seed}: round trip");
            assert_eq!(back.depth, g.depth);
            assert_eq!(back.player, g.player);
            assert_eq!(back.enemies, g.enemies);
            assert_eq!(back.pending, g.pending);
            assert_eq!(back.drops, g.drops);
            // And it carries on exactly as the original would have.
            let (mut a, mut b) = (g.clone(), back);
            let (mut ea, mut eb) = (Vec::new(), Vec::new());
            for _ in 0..600 {
                for (game, out) in [(&mut a, &mut ea), (&mut b, &mut eb)] {
                    if let Some(cmd) = crate::auto::next_command(game) {
                        game.apply(cmd, out);
                    }
                    game.tick(1.0 / 30.0, out);
                }
            }
            assert_eq!(ea, eb, "seed {seed}: diverged after loading");
        }
    }

    #[test]
    fn banked_level_ups_survive_a_save() {
        let mut g = played(3, 12000);
        g.player.hp = g.player.max_hp;
        if g.pending.is_empty() {
            g.pending
                .push(alloc::vec![Perk::Vigor, Perk::Scholar, Perk::Evasion]);
        }
        let mut back = Game::load(&g.save()).unwrap();
        let mut out = Vec::new();
        back.apply(Command::Choose(0), &mut out);
        assert!(matches!(out[0], Event::PerkTaken { .. }));
    }

    #[test]
    fn junk_is_not_a_save() {
        let good = played(9, 600).save();
        assert!(Game::load(b"").is_none());
        assert!(Game::load(b"ZRS0aaaaaaaaaaaaaaaaaaaaaaaaa").is_none());
        assert!(Game::load(&good[..good.len() - 1]).is_none());
        let mut extra = good.clone();
        extra.push(0);
        assert!(Game::load(&extra).is_none());
    }

    #[test]
    fn version_1_saves_still_load() {
        let g = played(5, 3000);
        let mut v1 = g.save();
        // Rewrite as version 1: old magic, no gold or drops after the perks.
        v1[..4].copy_from_slice(MAGIC_V1);
        let perks_end = 4 + 8 + 4 + 2 + 8 + 4 * 2 + 1 + 4 * 5 + 4 * 2 + 4 + Perk::ALL.len();
        // Gold, the bag and the quiver, then the drops with their items
        // and arrows, then the arrows in the air.
        let flights = 4 + 4 + 29 * g.flights.len();
        let gold_and_drops =
            4 + Item::ALL.len() + 8 + 4 + (16 + Item::ALL.len()) * g.drops.len() + flights;
        v1.drain(perks_end..perks_end + gold_and_drops);
        let back = Game::load(&v1).expect("version 1 loads");
        assert_eq!(back.player.gold, 0);
        assert!(back.drops.is_empty());
        assert_eq!(back.player.level, g.player.level);
    }
}
