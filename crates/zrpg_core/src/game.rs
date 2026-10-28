//! Game state and the loop the front-end drives.
//!
//! The front-end calls [`Game::apply`] for each player command and
//! [`Game::tick`] once per frame. Both resolve instantly and append
//! [`Event`]s, which the front-end animates at whatever pace it likes.

use alloc::vec::Vec;

use crate::archery::{Flight, Missile, tuning as arrow_tuning};
use crate::dungeon::{Floor, GenParams, Place};
use crate::enemy::{Enemy, EnemyId, EnemyKind, EnemyState, tuning as enemy_tuning};
use crate::equipment::Equipment;
use crate::grid::{Facing, GridPos, Step, Turn};
use crate::items::{Bag, Item, tuning as item_tuning};
use crate::map::Tile;
use crate::progression::{self, MAX_LEVEL, Perk, Perks};
use crate::rng::{Rng, mix64};
use crate::spells::{self, Spell, tuning as spell_tuning};
use crate::stage;

/// Player tuning, in seconds, hit points and energy points.
pub mod tuning {
    pub const MAX_HP: i32 = 20;
    pub const MAX_ENERGY: f32 = 10.0;
    /// Energy regained per second.
    pub const ENERGY_REGEN: f32 = 2.0;
    pub const ATTACK_COST: f32 = 3.0;
    pub const ATTACK_COOLDOWN: f32 = 0.5;
    pub const DAMAGE_MIN: i32 = 2;
    pub const DAMAGE_MAX: i32 = 4;
    /// Health restored on reaching a new floor: a small reward for pressing on.
    pub const DESCEND_HEAL: i32 = 5;
}

/// What each ability adds per time it is taken (see `progression.rs`).
mod perk_tuning {
    pub const VIGOR_HP: i32 = 4;
    pub const STAMINA_ENERGY: f32 = 2.0;
    pub const BREATH_REGEN: f32 = 0.25;
    pub const LIGHT_BLADE_COST: f32 = 0.4;
    pub const QUICK_HANDS_COOLDOWN: f32 = 0.05;
    pub const IRON_SKIN: i32 = 1;
    pub const EVASION_PERCENT: u32 = 6;
    pub const CRUSHING_PERCENT: u32 = 7;
    pub const BLOODTHIRST_HEAL: i32 = 1;
    /// Seconds per Health regenerated, divided by the stacks taken.
    pub const REGEN_SECS: f32 = 10.0;
    pub const SCHOLAR_XP_PERCENT: u32 = 15;
    pub const PILGRIM_HEAL: i32 = 3;
    pub const CONCUSSION_SECS: f32 = 0.25;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Step(Step),
    Turn(Turn),
    Attack,
    /// Take the stairs down. Only works while standing on them.
    Descend,
    /// Take ability `n` (0..3) of the oldest level-up still waiting.
    Choose(u8),
    /// Use one of an item from the bag.
    UseItem(Item),
    /// Cast a spell.
    Cast(Spell),
    /// Loose an arrow straight ahead.
    Shoot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Moved {
        from: GridPos,
        to: GridPos,
    },
    /// Tried to walk into a wall or an enemy.
    Bumped {
        at: GridPos,
        dir: Facing,
    },
    Turned {
        from: Facing,
        to: Facing,
        turn: Turn,
    },
    /// The player swung. `hit` names the enemy struck, if any.
    Swung {
        hit: Option<EnemyId>,
    },
    /// Tried to attack without enough energy.
    Exhausted,
    EnemyNoticed {
        id: EnemyId,
    },
    EnemyMoved {
        id: EnemyId,
        from: GridPos,
        to: GridPos,
    },
    EnemyWindUp {
        id: EnemyId,
    },
    /// A strike landed (`damage > 0`) or whiffed because the player moved.
    EnemyStruck {
        id: EnemyId,
        damage: i32,
    },
    EnemyHurt {
        id: EnemyId,
        damage: i32,
        staggered: bool,
    },
    EnemyDied {
        id: EnemyId,
        at: GridPos,
    },
    PlayerDied,
    /// Arrived on a freshly generated floor.
    Descended {
        depth: u32,
    },
    /// A kill earned XP.
    GainedXp {
        xp: u32,
    },
    /// Reached a new level; an ability choice is waiting.
    LeveledUp {
        level: u32,
    },
    /// An ability was taken.
    PerkTaken {
        perk: Perk,
    },
    /// A blow was dodged (Evasion).
    Dodged {
        id: EnemyId,
    },
    /// A kill left gold on the floor.
    GoldDropped {
        at: GridPos,
        amount: u32,
    },
    /// Walked onto gold and took it.
    GoldTaken {
        at: GridPos,
        amount: u32,
    },
    /// Picked up an item.
    ItemTaken {
        item: Item,
    },
    /// Used an item.
    ItemUsed {
        item: Item,
    },
    /// A spell went off from `from`; `reach` is where its line ends (the
    /// bolt) or its first target.
    SpellCast {
        spell: Spell,
        from: GridPos,
        reach: GridPos,
    },
    /// A spell struck an enemy at `at`; `order` counts its strikes (the
    /// sparks of a Spark, the jumps of a chain).
    SpellHit {
        spell: Spell,
        id: EnemyId,
        at: GridPos,
        order: u8,
    },
    /// Nothing for the spell to strike; no Chi spent.
    SpellFizzled {
        spell: Spell,
    },
    /// The psionic blade's last swing: it comes apart, and the sword is
    /// back in hand.
    BladeFaded,
    /// Something was loosed down the line: an arrow, a spark or a hammer
    /// (see `Game::flights`).
    Launched {
        id: u32,
        missile: Missile,
        from: GridPos,
        dir: Facing,
    },
    /// It struck an enemy at `at`.
    MissileHit {
        id: u32,
        missile: Missile,
        enemy: EnemyId,
        at: GridPos,
    },
    /// An arrow fell at `at`: against a wall (`wall`), or spent. `kept` if
    /// it lies there whole, to be picked up.
    ArrowLanded {
        id: u32,
        at: GridPos,
        wall: bool,
        kept: bool,
    },
    /// A spark went out at `at`, against a wall or spent.
    SparkSpent {
        id: u32,
        at: GridPos,
    },
    /// A hammer turned for home at `at`.
    HammerTurned {
        id: u32,
        at: GridPos,
    },
    /// A hammer came back to your hand.
    HammerCaught {
        id: u32,
    },
    /// Tried to shoot with an empty quiver.
    OutOfArrows,
    /// Picked up arrows.
    ArrowsTaken {
        amount: u32,
    },
}

/// A pool of gold, and perhaps items, where something vanished.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Drop {
    pub pos: GridPos,
    pub gold: u32,
    pub items: Bag,
    pub arrows: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Player {
    pub pos: GridPos,
    pub facing: Facing,
    pub hp: i32,
    pub max_hp: i32,
    pub energy: f32,
    pub max_energy: f32,
    /// Seconds until the next attack is allowed.
    pub attack_cooldown: f32,
    /// 1 to [`MAX_LEVEL`].
    pub level: u32,
    /// XP towards the next level.
    pub xp: u32,
    pub perks: Perks,
    /// Gold carried.
    pub gold: u32,
    /// Items carried.
    pub bag: Bag,
    /// What you wear and hold.
    pub equipment: Equipment,
    /// Arrows in the quiver, and seconds until the bow can shoot again.
    pub arrows: u32,
    pub bow_cooldown: f32,
    /// Swings left in a conjured psionic blade (0: the sword).
    pub blade: u8,
    /// Seconds until each spell can be cast again, in `Spell::ALL` order.
    pub spell_cooldowns: [f32; Spell::ALL.len()],
    /// Seconds towards the next Health from Regeneration.
    pub(crate) regen: f32,
}

impl Player {
    fn new(pos: GridPos, facing: Facing) -> Self {
        Self {
            pos,
            facing,
            hp: tuning::MAX_HP,
            max_hp: tuning::MAX_HP,
            energy: tuning::MAX_ENERGY,
            max_energy: tuning::MAX_ENERGY,
            attack_cooldown: 0.0,
            level: 1,
            xp: 0,
            perks: Perks::default(),
            gold: 0,
            bag: Bag::default(),
            equipment: Equipment::starting_kit(),
            arrows: arrow_tuning::START_ARROWS,
            bow_cooldown: 0.0,
            blade: 0,
            spell_cooldowns: [0.0; Spell::ALL.len()],
            regen: 0.0,
        }
    }

    fn stacks(&self, perk: Perk) -> u8 {
        self.perks.count(perk)
    }

    pub fn attack_cost(&self) -> f32 {
        tuning::ATTACK_COST - perk_tuning::LIGHT_BLADE_COST * self.stacks(Perk::LightBlade) as f32
    }

    pub fn attack_cooldown_secs(&self) -> f32 {
        tuning::ATTACK_COOLDOWN
            - perk_tuning::QUICK_HANDS_COOLDOWN * self.stacks(Perk::QuickHands) as f32
    }

    fn energy_regen(&self) -> f32 {
        tuning::ENERGY_REGEN
            * (1.0 + perk_tuning::BREATH_REGEN * self.stacks(Perk::SecondBreath) as f32)
    }

    /// XP still needed for the next level (0 at the top).
    pub fn xp_needed(&self) -> u32 {
        if self.level >= MAX_LEVEL {
            0
        } else {
            progression::xp_to_next(self.level)
        }
    }

    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }

    /// The least and most a swing does, before Crushing Blow: the sword's,
    /// or the psionic blade's while it lasts.
    pub fn swing_damage(&self) -> (i32, i32) {
        let edge = self.stacks(Perk::KeenEdge) as i32;
        if self.blade > 0 {
            let bonus = spells::level_bonus(self.level);
            (
                spell_tuning::BLADE_DAMAGE_MIN + edge + bonus,
                spell_tuning::BLADE_DAMAGE_MAX + edge + bonus,
            )
        } else {
            (tuning::DAMAGE_MIN + edge, tuning::DAMAGE_MAX + edge)
        }
    }

    /// Ready to swing and able to pay for it (the psionic blade is free).
    pub fn can_attack(&self) -> bool {
        self.attack_cooldown <= 0.0 && (self.blade > 0 || self.energy >= self.attack_cost())
    }
}

#[derive(Clone, Debug)]
pub struct Game {
    pub run_seed: u64,
    /// 1-based floor number.
    pub depth: u32,
    pub params: GenParams,
    pub floor: Floor,
    pub player: Player,
    pub enemies: Vec<Enemy>,
    /// Gold lying on this floor.
    pub drops: Vec<Drop>,
    /// Level-ups waiting for an ability to be chosen, oldest first: each is
    /// the three abilities on offer.
    pub pending: Vec<Vec<Perk>>,
    /// Arrows in the air.
    pub flights: Vec<Flight>,
    pub(crate) next_flight: u32,
    pub(crate) rng: Rng,
}

impl Game {
    pub fn new(run_seed: u64) -> Self {
        Self::with_params(run_seed, GenParams::default())
    }

    pub fn with_params(run_seed: u64, params: GenParams) -> Self {
        let floor = Floor::for_depth(run_seed, 1, &params);
        let player = Player::new(floor.start, floor.start_facing);
        let enemies = spawn_enemies(&floor, run_seed, 1);
        Self {
            run_seed,
            depth: 1,
            params,
            floor,
            player,
            enemies,
            drops: Vec::new(),
            pending: Vec::new(),
            flights: Vec::new(),
            next_flight: 0,
            rng: Rng::new(mix64(run_seed ^ 0xC0FF_EE00)),
        }
    }

    pub fn on_stairs(&self) -> bool {
        self.floor.map.get(self.player.pos) == Tile::StairsDown
    }

    /// Generate the next floor and place the player at its start. Health,
    /// Energy and the run's RNG carry over.
    fn descend(&mut self) {
        self.depth += 1;
        self.floor = Floor::for_depth(self.run_seed, self.depth, &self.params);
        self.enemies = spawn_enemies(&self.floor, self.run_seed, self.depth);
        // Gold left behind stays behind, as do arrows in the air.
        self.drops.clear();
        self.flights.clear();
        let p = &mut self.player;
        p.pos = self.floor.start;
        p.facing = self.floor.start_facing;
        let heal =
            tuning::DESCEND_HEAL + perk_tuning::PILGRIM_HEAL * p.stacks(Perk::Pilgrim) as i32;
        p.hp = (p.hp + heal).min(p.max_hp);
    }

    /// Earn XP; level up (each level queues an ability choice).
    fn gain_xp(&mut self, base: u32, out: &mut Vec<Event>) {
        let p = &mut self.player;
        let scholar = 100 + perk_tuning::SCHOLAR_XP_PERCENT * p.stacks(Perk::Scholar) as u32;
        let xp = base * scholar / 100;
        out.push(Event::GainedXp { xp });
        if p.level >= MAX_LEVEL {
            return;
        }
        p.xp += xp;
        while self.player.level < MAX_LEVEL && self.player.xp >= self.player.xp_needed() {
            let p = &mut self.player;
            p.xp -= p.xp_needed();
            p.level += 1;
            // Offers are rolled now, with everything already chosen or still
            // waiting counted, so no offer can overflow an ability's cap.
            let mut planned = self.player.perks;
            for offer in &self.pending {
                planned.add(offer[0]);
            }
            let offer = progression::roll_offer(&mut self.rng, &planned);
            self.pending.push(offer);
            out.push(Event::LeveledUp {
                level: self.player.level,
            });
        }
        if self.player.level >= MAX_LEVEL {
            self.player.xp = 0;
        }
    }

    /// Take the pool of gold and items at `at`, as much as the bag holds.
    fn pick_up(&mut self, at: GridPos, out: &mut Vec<Event>) {
        let Some(i) = self.drops.iter().position(|d| d.pos == at) else {
            return;
        };
        let drop = &mut self.drops[i];
        if drop.gold > 0 {
            let amount = core::mem::take(&mut drop.gold);
            self.player.gold += amount;
            out.push(Event::GoldTaken { at, amount });
        }
        let room = arrow_tuning::MAX_ARROWS.saturating_sub(self.player.arrows);
        let amount = drop.arrows.min(room);
        if amount > 0 {
            drop.arrows -= amount;
            self.player.arrows += amount;
            out.push(Event::ArrowsTaken { amount });
        }
        for item in Item::ALL {
            while drop.items.count(item) > 0 && self.player.bag.add(item) {
                drop.items.take(item);
                out.push(Event::ItemTaken { item });
            }
        }
        // What the bag can't hold stays where it lies.
        if drop.gold == 0 && drop.items.is_empty() && drop.arrows == 0 {
            self.drops.swap_remove(i);
        }
    }

    /// Use one `item` from the bag.
    fn use_item(&mut self, item: Item, out: &mut Vec<Event>) {
        if !self.player.bag.take(item) {
            return;
        }
        let p = &mut self.player;
        match item {
            Item::HealthPotion => p.hp = (p.hp + item_tuning::POTION_HEAL).min(p.max_hp),
        }
        out.push(Event::ItemUsed { item });
    }

    /// Take ability `choice` of the oldest waiting level-up.
    fn choose(&mut self, choice: usize, out: &mut Vec<Event>) {
        if self.pending.is_empty() {
            return;
        }
        let Some(&perk) = self.pending[0].get(choice) else {
            return;
        };
        if !self.player.perks.can_take(perk) {
            return;
        }
        self.pending.remove(0);
        let p = &mut self.player;
        p.perks.add(perk);
        match perk {
            Perk::Vigor => {
                p.max_hp += perk_tuning::VIGOR_HP;
                p.hp += perk_tuning::VIGOR_HP;
            }
            Perk::Stamina => {
                p.max_energy += perk_tuning::STAMINA_ENERGY;
                p.energy += perk_tuning::STAMINA_ENERGY;
            }
            _ => {}
        }
        out.push(Event::PerkTaken { perk });
        // Offers still waiting were rolled guessing at earlier picks; swap
        // out anything that can no longer be taken.
        let perks = self.player.perks;
        for i in 0..self.pending.len() {
            self.pending[i].retain(|&p| perks.can_take(p));
            while self.pending[i].len() < 3 {
                let offer = &self.pending[i];
                let fresh: Vec<Perk> = Perk::ALL
                    .into_iter()
                    .filter(|p| perks.can_take(*p) && !offer.contains(p))
                    .collect();
                if fresh.is_empty() {
                    break;
                }
                let k = self.rng.range(0, fresh.len() as i32 - 1) as usize;
                self.pending[i].push(fresh[k]);
            }
        }
    }

    pub fn enemy_at(&self, p: GridPos) -> Option<&Enemy> {
        self.enemies.iter().find(|e| e.pos == p)
    }

    /// Can something at `from` step `dir` (walkable, edge open, unoccupied)?
    fn is_open(&self, from: GridPos, dir: Facing) -> bool {
        self.floor.map.can_step(from, dir) && self.enemy_at(from + dir).is_none()
    }

    /// Resolve one player command. The dead don't take orders.
    pub fn apply(&mut self, cmd: Command, out: &mut Vec<Event>) {
        if !self.player.is_alive() {
            return;
        }
        match cmd {
            Command::Step(step) => {
                let dir = step.world_dir(self.player.facing);
                let from = self.player.pos;
                let to = from + dir;
                if self.is_open(from, dir) {
                    self.player.pos = to;
                    out.push(Event::Moved { from, to });
                    self.pick_up(to, out);
                } else {
                    out.push(Event::Bumped { at: from, dir });
                }
            }
            Command::Turn(turn) => {
                let from = self.player.facing;
                self.player.facing = from.turned(turn);
                out.push(Event::Turned {
                    from,
                    to: self.player.facing,
                    turn,
                });
            }
            Command::Attack => self.attack(out),
            Command::Descend => {
                if self.on_stairs() {
                    self.descend();
                    out.push(Event::Descended { depth: self.depth });
                }
            }
            Command::Choose(n) => self.choose(n as usize, out),
            Command::UseItem(item) => self.use_item(item, out),
            Command::Cast(spell) => self.cast(spell, out),
            Command::Shoot => self.shoot(out),
        }
    }

    fn attack(&mut self, out: &mut Vec<Event>) {
        let p = &mut self.player;
        if p.attack_cooldown > 0.0 {
            return;
        }
        let psionic = p.blade > 0;
        if !psionic && p.energy < p.attack_cost() {
            out.push(Event::Exhausted);
            return;
        }
        if !psionic {
            p.energy -= p.attack_cost();
        }
        p.attack_cooldown = p.attack_cooldown_secs();
        let (low, high) = p.swing_damage();
        // Every swing wears the blade, hit or miss.
        p.blade = p.blade.saturating_sub(1);
        let fades = psionic && p.blade == 0;
        let crushing = p.stacks(Perk::CrushingBlow) as u32 * perk_tuning::CRUSHING_PERCENT;
        let target = p.pos + p.facing;
        // A parapet between you and it is out of reach.
        let reachable = self.floor.map.within_reach(p.pos, target);

        let Some(i) = self
            .enemies
            .iter()
            .position(|e| e.pos == target && reachable)
        else {
            out.push(Event::Swung { hit: None });
            if fades {
                out.push(Event::BladeFaded);
            }
            return;
        };
        let mut damage = self.rng.range(low, high);
        if crushing > 0 && self.rng.chance(crushing) {
            damage *= 2;
        }
        out.push(Event::Swung {
            hit: Some(self.enemies[i].id),
        });
        self.hurt_enemy(i, damage, out);
        if fades {
            out.push(Event::BladeFaded);
        }
    }

    /// Hurt enemy `i`: a hit wakes it, a hit during its wind-up staggers
    /// it, and a killing hit leaves loot and gives XP. Returns whether it
    /// died (and so left the list).
    fn hurt_enemy(&mut self, i: usize, damage: i32, out: &mut Vec<Event>) -> bool {
        let concussion = self.player.stacks(Perk::Concussion) as f32 * perk_tuning::CONCUSSION_SECS;
        let enemy = &mut self.enemies[i];
        enemy.hp -= damage;
        let staggered = enemy.is_winding_up();
        if staggered {
            enemy.state = EnemyState::Recover {
                left: enemy_tuning::STAGGER + concussion,
            };
        } else if enemy.state == EnemyState::Dormant {
            enemy.state = EnemyState::Chasing;
        }
        out.push(Event::EnemyHurt {
            id: enemy.id,
            damage,
            staggered,
        });
        if enemy.hp > 0 {
            return false;
        }
        out.push(Event::EnemyDied {
            id: enemy.id,
            at: enemy.pos,
        });
        let (kind, at) = (enemy.kind, enemy.pos);
        self.enemies.swap_remove(i);
        let amount = progression::gold_for_kill(kind, self.depth, self.rng.next_u32());
        let mut items = Bag::default();
        for item in Item::ALL {
            if self.rng.chance(item.drop_percent(kind)) {
                items.add(item);
            }
        }
        let arrows = if self.rng.chance(arrow_tuning::DROP_PERCENT) {
            self.rng
                .range(arrow_tuning::DROP_MIN, arrow_tuning::DROP_MAX) as u32
        } else {
            0
        };
        self.leave(Drop {
            pos: at,
            gold: amount,
            items,
            arrows,
        });
        out.push(Event::GoldDropped { at, amount });
        let p = &mut self.player;
        let heal = perk_tuning::BLOODTHIRST_HEAL * p.stacks(Perk::Bloodthirst) as i32;
        p.hp = (p.hp + heal).min(p.max_hp);
        self.gain_xp(progression::xp_for_kill(kind, self.depth), out);
        true
    }

    /// Add `drop` to whatever already lies at its cell.
    fn leave(&mut self, drop: Drop) {
        match self.drops.iter_mut().find(|d| d.pos == drop.pos) {
            Some(d) => {
                d.gold += drop.gold;
                d.arrows += drop.arrows;
                for (item, n) in drop.items.items() {
                    for _ in 0..n {
                        d.items.add(item);
                    }
                }
            }
            None => self.drops.push(drop),
        }
    }

    /// Loose an arrow, if the bow is ready and there is an arrow.
    fn shoot(&mut self, out: &mut Vec<Event>) {
        let p = &mut self.player;
        if p.bow_cooldown > 0.0 || p.equipment.get(crate::Slot::Bow).is_none() {
            return;
        }
        if p.arrows == 0 {
            out.push(Event::OutOfArrows);
            return;
        }
        p.arrows -= 1;
        p.bow_cooldown = arrow_tuning::COOLDOWN;
        self.launch(Missile::Arrow, out);
    }

    /// Send `missile` down the line from where you stand.
    fn launch(&mut self, missile: Missile, out: &mut Vec<Event>) {
        let (from, dir) = (self.player.pos, self.player.facing);
        let id = self.next_flight;
        self.next_flight = self.next_flight.wrapping_add(1);
        self.flights.push(Flight::new(id, missile, from, dir));
        out.push(Event::Launched {
            id,
            missile,
            from,
            dir,
        });
    }

    /// Hammers in the air (going out or coming home).
    pub fn hammers_out(&self) -> usize {
        self.flights
            .iter()
            .filter(|f| f.missile == Missile::Hammer)
            .count()
    }

    /// The first enemy an arrow loosed now would meet, if it stood still.
    pub fn arrow_target(&self) -> Option<&Enemy> {
        let map = &self.floor.map;
        let (mut at, dir) = (self.player.pos, self.player.facing);
        for _ in 0..arrow_tuning::RANGE {
            if !map.can_step(at, dir) {
                return None;
            }
            at = at + dir;
            if let Some(e) = self.enemy_at(at) {
                return Some(e);
            }
        }
        None
    }

    /// Move everything in the air on by `dt`: into the next cells, until
    /// it meets an enemy, a wall or the end of its range. Hammers then
    /// fly home.
    fn fly_missiles(&mut self, dt: f32, out: &mut Vec<Event>) {
        let mut i = 0;
        while i < self.flights.len() {
            let mut f = self.flights[i];
            let step = f.missile.speed() * dt;
            let mut done = false;
            if f.returning {
                // Home along the way it came; nothing stops it now.
                f.dist -= step;
                if f.dist <= 0.0 {
                    out.push(Event::HammerCaught { id: f.id });
                    done = true;
                }
            } else {
                f.dist += step;
            }
            while !done && !f.returning && f.entered < f.cells_reached() {
                if !self.floor.map.can_step(f.cell, f.dir) {
                    done = self.missile_stops(&mut f, true, out);
                    continue;
                }
                f.cell = f.cell + f.dir;
                f.entered += 1;
                if let Some(e) = self.enemies.iter().position(|e| e.pos == f.cell) {
                    self.missile_strikes(&mut f, e, out);
                    done = f.missile != Missile::Hammer;
                } else if f.entered >= f.missile.range() {
                    done = self.missile_stops(&mut f, false, out);
                }
            }
            if done {
                self.flights.remove(i);
            } else {
                self.flights[i] = f;
                i += 1;
            }
        }
    }

    /// `f` strikes enemy `e`. A hammer turns for home from here.
    fn missile_strikes(&mut self, f: &mut Flight, e: usize, out: &mut Vec<Event>) {
        out.push(Event::MissileHit {
            id: f.id,
            missile: f.missile,
            enemy: self.enemies[e].id,
            at: f.cell,
        });
        let bonus = spells::level_bonus(self.player.level);
        let damage = match f.missile {
            Missile::Arrow => {
                self.rng
                    .range(arrow_tuning::DAMAGE_MIN, arrow_tuning::DAMAGE_MAX)
                    + bonus
            }
            Missile::Spark => spell_tuning::BARRAGE_DAMAGE + bonus / 2,
            Missile::Hammer => {
                self.rng.range(
                    spell_tuning::HAMMER_DAMAGE_MIN,
                    spell_tuning::HAMMER_DAMAGE_MAX,
                ) + bonus
            }
        };
        self.hurt_enemy(e, damage, out);
        match f.missile {
            Missile::Arrow if self.rng.chance(arrow_tuning::KEPT_HIT_PERCENT) => {
                self.leave(Drop {
                    pos: f.cell,
                    gold: 0,
                    items: Bag::default(),
                    arrows: 1,
                });
            }
            Missile::Hammer => self.turn_home(f, out),
            _ => {}
        }
    }

    /// `f` can go no further (a wall, or the end of its reach). Returns
    /// whether it is gone; a hammer instead turns for home.
    fn missile_stops(&mut self, f: &mut Flight, wall: bool, out: &mut Vec<Event>) -> bool {
        match f.missile {
            Missile::Arrow => {
                self.arrow_falls(f.id, f.cell, wall, out);
                true
            }
            Missile::Spark => {
                out.push(Event::SparkSpent {
                    id: f.id,
                    at: f.cell,
                });
                true
            }
            Missile::Hammer => {
                self.turn_home(f, out);
                false
            }
        }
    }

    /// A hammer starts back from the middle of the cell it reached.
    fn turn_home(&mut self, f: &mut Flight, out: &mut Vec<Event>) {
        f.returning = true;
        f.dist = f.entered as f32;
        out.push(Event::HammerTurned {
            id: f.id,
            at: f.cell,
        });
    }

    /// An arrow comes down at `at`, whole or broken.
    fn arrow_falls(&mut self, id: u32, at: GridPos, wall: bool, out: &mut Vec<Event>) {
        let kept = self.rng.chance(arrow_tuning::KEPT_MISS_PERCENT);
        if kept {
            self.leave(Drop {
                pos: at,
                gold: 0,
                items: Bag::default(),
                arrows: 1,
            });
        }
        out.push(Event::ArrowLanded { id, at, wall, kept });
        // Shot point-blank into a wall: it falls at your feet, and you take
        // it back.
        if at == self.player.pos {
            self.pick_up(at, out);
        }
    }

    /// The enemies a spell would strike, in order, and where its line
    /// ends (for the bolt). Empty if it has nothing to strike.
    pub fn spell_targets(&self, spell: Spell) -> (Vec<EnemyId>, GridPos) {
        let map = &self.floor.map;
        let p = &self.player;
        // Cells straight ahead, until a wall, a closed edge, or the range.
        let ahead = |range: i32| {
            let mut cells = Vec::new();
            let mut at = p.pos;
            for _ in 0..range {
                if !map.can_step(at, p.facing) {
                    break;
                }
                at = at + p.facing;
                cells.push(at);
            }
            cells
        };
        match spell {
            Spell::Spark => {
                let cells = ahead(spell_tuning::SPARK_RANGE);
                let hit = cells
                    .iter()
                    .find_map(|&c| self.enemy_at(c).map(|e| (e.id, c)));
                match hit {
                    Some((id, c)) => (alloc::vec![id], c),
                    None => (Vec::new(), cells.last().copied().unwrap_or(p.pos)),
                }
            }
            Spell::LightningBolt => {
                let cells = ahead(spell_tuning::BOLT_RANGE);
                let ids = cells
                    .iter()
                    .filter_map(|&c| self.enemy_at(c).map(|e| e.id))
                    .collect();
                (ids, cells.last().copied().unwrap_or(p.pos))
            }
            Spell::ChainLightning => {
                let mut ids: Vec<EnemyId> = Vec::new();
                let mut from = p.pos;
                let mut reach = spell_tuning::CHAIN_RANGE;
                while ids.len() < spell_tuning::CHAIN_TARGETS {
                    let next = self
                        .enemies
                        .iter()
                        .filter(|e| !ids.contains(&e.id))
                        .filter(|e| e.pos.manhattan(from) <= reach && map.in_sight(from, e.pos))
                        .min_by_key(|e| (e.pos.manhattan(from), e.id));
                    let Some(e) = next else { break };
                    ids.push(e.id);
                    from = e.pos;
                    reach = spell_tuning::CHAIN_JUMP;
                }
                let first = ids
                    .first()
                    .and_then(|&id| self.enemies.iter().find(|e| e.id == id))
                    .map_or(p.pos, |e| e.pos);
                (ids, first)
            }
            // These aim themselves (see `cast`).
            Spell::PsionicBlade | Spell::MagicHammer | Spell::SparkBarrage => (Vec::new(), p.pos),
        }
    }

    /// Cast `spell`, if it is ready, there is Chi for it, and (for Spark
    /// and Chain Lightning) something to strike.
    fn cast(&mut self, spell: Spell, out: &mut Vec<Event>) {
        let p = &self.player;
        if p.spell_cooldowns[spell.index()] > 0.0 {
            return;
        }
        if p.energy < spell.cost() {
            out.push(Event::Exhausted);
            return;
        }
        if matches!(
            spell,
            Spell::PsionicBlade | Spell::MagicHammer | Spell::SparkBarrage
        ) {
            // Nothing to aim at first: the blade forms in your hand (afresh,
            // if one is already there); a hammer or a spark flies down the
            // line and finds its own mark.
            if spell == Spell::MagicHammer && self.hammers_out() >= spell_tuning::HAMMERS {
                return;
            }
            let p = &mut self.player;
            p.energy -= spell.cost();
            p.spell_cooldowns[spell.index()] = spell.cooldown();
            let from = p.pos;
            out.push(Event::SpellCast {
                spell,
                from,
                reach: from,
            });
            match spell {
                Spell::PsionicBlade => self.player.blade = spell_tuning::BLADE_SWINGS,
                Spell::MagicHammer => self.launch(Missile::Hammer, out),
                _ => self.launch(Missile::Spark, out),
            }
            return;
        }
        let (targets, reach) = self.spell_targets(spell);
        if targets.is_empty() && spell != Spell::LightningBolt {
            out.push(Event::SpellFizzled { spell });
            return;
        }
        let p = &mut self.player;
        p.energy -= spell.cost();
        p.spell_cooldowns[spell.index()] = spell.cooldown();
        let bonus = spells::level_bonus(p.level);
        out.push(Event::SpellCast {
            spell,
            from: p.pos,
            reach,
        });
        let hits: Vec<(EnemyId, i32)> = match spell {
            Spell::Spark => {
                let n = self
                    .rng
                    .range(spell_tuning::SPARK_HITS_MIN, spell_tuning::SPARK_HITS_MAX);
                (0..n)
                    .map(|_| (targets[0], spell_tuning::SPARK_DAMAGE + bonus))
                    .collect()
            }
            Spell::ChainLightning => targets
                .iter()
                .zip(spell_tuning::CHAIN_DAMAGE)
                .map(|(&id, d)| (id, d + bonus))
                .collect(),
            Spell::LightningBolt => targets
                .iter()
                .map(|&id| (id, spell_tuning::BOLT_DAMAGE + bonus))
                .collect(),
            // Handled above.
            Spell::PsionicBlade | Spell::MagicHammer | Spell::SparkBarrage => Vec::new(),
        };
        for (order, (id, damage)) in hits.into_iter().enumerate() {
            // A target already dead (Spark's later sparks) is skipped.
            let Some(i) = self.enemies.iter().position(|e| e.id == id) else {
                continue;
            };
            let at = self.enemies[i].pos;
            out.push(Event::SpellHit {
                spell,
                id,
                at,
                order: order as u8,
            });
            self.hurt_enemy(i, damage, out);
        }
    }

    /// Advance real time by `dt` seconds.
    pub fn tick(&mut self, dt: f32, out: &mut Vec<Event>) {
        if !self.player.is_alive() {
            return;
        }
        let p = &mut self.player;
        p.attack_cooldown = (p.attack_cooldown - dt).max(0.0);
        p.bow_cooldown = (p.bow_cooldown - dt).max(0.0);
        for c in p.spell_cooldowns.iter_mut() {
            *c = (*c - dt).max(0.0);
        }
        p.energy = (p.energy + p.energy_regen() * dt).min(p.max_energy);
        let regen = p.stacks(Perk::Regeneration);
        if regen > 0 && p.hp < p.max_hp {
            p.regen += dt * regen as f32;
            if p.regen >= perk_tuning::REGEN_SECS {
                p.regen -= perk_tuning::REGEN_SECS;
                p.hp += 1;
            }
        }

        self.fly_missiles(dt, out);
        if !self.player.is_alive() {
            return;
        }

        if self.enemies.is_empty() {
            return;
        }
        // One flow field from the player serves every enemy this tick.
        let dist = self.floor.map.distances(self.player.pos);
        for i in 0..self.enemies.len() {
            self.tick_enemy(i, dt, &dist, out);
            if !self.player.is_alive() {
                out.push(Event::PlayerDied);
                return;
            }
        }
    }

    fn tick_enemy(&mut self, i: usize, dt: f32, dist: &[Option<u32>], out: &mut Vec<Event>) {
        let map = &self.floor.map;
        let player_pos = self.player.pos;
        let adjacent = map.within_reach(self.enemies[i].pos, player_pos);
        let stats = self.enemies[i].stats();
        let walk_dist = dist[map.index(self.enemies[i].pos)];
        let id = self.enemies[i].id;

        match self.enemies[i].state {
            EnemyState::Dormant => {
                if walk_dist.is_some_and(|d| d <= stats.notice_range) {
                    self.enemies[i].state = EnemyState::Chasing;
                    out.push(Event::EnemyNoticed { id });
                }
            }
            EnemyState::Chasing if adjacent => {
                self.enemies[i].state = EnemyState::WindUp {
                    left: stats.wind_up,
                };
                out.push(Event::EnemyWindUp { id });
            }
            EnemyState::Chasing => {
                let e = &mut self.enemies[i];
                e.step_timer -= dt;
                if e.step_timer > 0.0 {
                    return;
                }
                e.step_timer = stats.step_interval;
                let from = e.pos;
                // Step to the open neighbour closest to the player.
                let next = Facing::ALL
                    .into_iter()
                    .filter(|&d| self.is_open(from, d))
                    .map(|d| from + d)
                    .filter(|&n| n != player_pos)
                    // Stairs are for the player; enemies stay on solid ground.
                    .filter(|&n| map.get(n) != Tile::StairsDown)
                    .filter_map(|n| dist[map.index(n)].map(|d| (d, n)))
                    .filter(|&(d, _)| walk_dist.is_none_or(|w| d < w))
                    .min_by_key(|&(d, _)| d);
                if let Some((_, to)) = next {
                    self.enemies[i].pos = to;
                    out.push(Event::EnemyMoved { id, from, to });
                }
            }
            EnemyState::WindUp { left } => {
                let left = left - dt;
                if left > 0.0 {
                    self.enemies[i].state = EnemyState::WindUp { left };
                    return;
                }
                // Stepping away during the tell dodges the blow; so, now and
                // then, does Evasion.
                let p = &self.player;
                let evasion = p.stacks(Perk::Evasion) as u32 * perk_tuning::EVASION_PERCENT;
                let iron = perk_tuning::IRON_SKIN * p.stacks(Perk::IronSkin) as i32;
                let damage = if !adjacent {
                    0
                } else if evasion > 0 && self.rng.chance(evasion) {
                    out.push(Event::Dodged { id });
                    0
                } else {
                    (self.rng.range(stats.damage_min, stats.damage_max) - iron).max(1)
                };
                self.player.hp = (self.player.hp - damage).max(0);
                self.enemies[i].state = EnemyState::Recover {
                    left: stats.recover,
                };
                out.push(Event::EnemyStruck { id, damage });
            }
            EnemyState::Recover { left } => {
                let left = left - dt;
                self.enemies[i].state = if left > 0.0 {
                    EnemyState::Recover { left }
                } else {
                    EnemyState::Chasing
                };
            }
        }
    }
}

fn spawn_enemies(floor: &Floor, run_seed: u64, depth: u32) -> Vec<Enemy> {
    floor
        .enemy_spawns
        .iter()
        .enumerate()
        .map(|(i, &p)| {
            let roll = mix64(run_seed ^ mix64(depth as u64) ^ (i as u64) << 48);
            let kind = match floor.stage {
                Some(s) => stage::kind_for(s.theme, depth, roll),
                None => kind_for(floor.place, depth, roll),
            };
            Enemy::of_kind(i as EnemyId, p, kind)
        })
        .collect()
}

/// Who lives where: the undead haunt the ruins. Below, each floor draws from
/// every kind that has turned up by that depth, so deeper floors mix the
/// old with the new.
fn kind_for(place: Place, depth: u32, roll: u64) -> EnemyKind {
    if place == Place::Ruins {
        return EnemyKind::Undead;
    }
    let pool: Vec<EnemyKind> = EnemyKind::ALL
        .into_iter()
        .filter(|k| k.first_depth() <= depth.max(2))
        .collect();
    pool[(roll % pool.len() as u64) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::Item;
    use crate::map::Map;

    const DT: f32 = 1.0 / 60.0;

    /// A straight east-west corridor: player at x=1 facing east, enemy at `enemy_x`.
    fn corridor(enemy_x: i32) -> Game {
        let mut g = Game::new(0);
        let mut map = Map::filled(12, 3, Tile::Wall);
        for x in 1..11 {
            map.set(GridPos::new(x, 1), Tile::Floor);
        }
        g.floor.map = map;
        g.player = Player::new(GridPos::new(1, 1), Facing::East);
        g.enemies = alloc::vec![Enemy::new(7, GridPos::new(enemy_x, 1))];
        g
    }

    fn run(g: &mut Game, seconds: f32) -> Vec<Event> {
        let mut out = Vec::new();
        for _ in 0..(seconds / DT) as usize {
            g.tick(DT, &mut out);
        }
        out
    }

    #[test]
    fn walls_and_enemies_block_movement() {
        let mut g = corridor(2);
        let mut out = Vec::new();
        g.apply(Command::Step(Step::Forward), &mut out);
        assert!(matches!(out[0], Event::Bumped { .. }));
        assert_eq!(g.player.pos, GridPos::new(1, 1));
        out.clear();
        g.apply(Command::Step(Step::StrafeLeft), &mut out);
        assert!(matches!(out[0], Event::Bumped { .. }));
    }

    #[test]
    fn random_walk_never_leaves_floor() {
        let mut rng = Rng::new(1);
        let mut out = Vec::new();
        for seed in 0..50 {
            let mut g = Game::new(seed);
            for _ in 0..500 {
                let cmd = match rng.range(0, 6) {
                    0 => Command::Step(Step::Forward),
                    1 => Command::Step(Step::Back),
                    2 => Command::Step(Step::StrafeLeft),
                    3 => Command::Step(Step::StrafeRight),
                    4 => Command::Turn(Turn::Left),
                    5 => Command::Turn(Turn::Right),
                    _ => Command::Attack,
                };
                g.apply(cmd, &mut out);
                g.tick(0.05, &mut out);
                assert!(g.floor.map.is_walkable(g.player.pos));
                for e in &g.enemies {
                    assert!(g.floor.map.is_walkable(e.pos));
                    assert_ne!(e.pos, g.player.pos, "seed {seed}: enemy on player");
                    assert_ne!(g.floor.map.get(e.pos), Tile::StairsDown, "seed {seed}");
                }
            }
        }
    }

    #[test]
    fn enemy_notices_chases_and_strikes() {
        let mut g = corridor(6);
        let events = run(&mut g, 4.0);
        assert!(events.contains(&Event::EnemyNoticed { id: 7 }));
        assert!(events.iter().any(|e| matches!(e, Event::EnemyMoved { .. })));
        assert_eq!(g.enemies[0].pos, GridPos::new(2, 1), "should end adjacent");
        assert!(events.contains(&Event::EnemyWindUp { id: 7 }));
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::EnemyStruck { damage, .. } if *damage > 0))
        );
        assert!(g.player.hp < tuning::MAX_HP);
    }

    #[test]
    fn far_enemy_sleeps() {
        let mut g = corridor(10); // 9 cells away, beyond notice range
        let events = run(&mut g, 2.0);
        assert!(events.is_empty());
        assert_eq!(g.enemies[0].state, EnemyState::Dormant);
    }

    #[test]
    fn stepping_back_dodges_the_wind_up() {
        let mut g = corridor(3);
        g.player.pos = GridPos::new(2, 1);
        run(&mut g, DT * 2.0);
        assert!(g.enemies[0].is_winding_up());
        let mut out = Vec::new();
        g.apply(Command::Step(Step::Back), &mut out);
        assert_eq!(g.player.pos, GridPos::new(1, 1));
        let events = run(&mut g, enemy_tuning::WIND_UP);
        assert!(events.contains(&Event::EnemyStruck { id: 7, damage: 0 }));
        assert_eq!(g.player.hp, tuning::MAX_HP);
    }

    #[test]
    fn attacks_cost_energy_respect_cooldown_and_kill() {
        let mut g = corridor(2);
        g.enemies[0].state = EnemyState::Recover { left: 100.0 }; // hold still
        let mut out = Vec::new();
        g.apply(Command::Attack, &mut out);
        assert_eq!(out[0], Event::Swung { hit: Some(7) });
        assert_eq!(g.player.energy, tuning::MAX_ENERGY - tuning::ATTACK_COST);
        // Cooldown: an immediate second swing does nothing.
        out.clear();
        g.apply(Command::Attack, &mut out);
        assert!(out.is_empty());
        // Keep swinging (waiting out cooldowns) until it dies.
        let mut swings = 1;
        while !g.enemies.is_empty() {
            out.clear();
            g.tick(tuning::ATTACK_COOLDOWN + 1.0, &mut out);
            g.apply(Command::Attack, &mut out);
            swings += 1;
            assert!(swings < 10);
        }
        assert!(out.contains(&Event::EnemyDied {
            id: 7,
            at: GridPos::new(2, 1)
        }));
        // The corpse no longer blocks the corridor.
        out.clear();
        g.apply(Command::Step(Step::Forward), &mut out);
        assert_eq!(g.player.pos, GridPos::new(2, 1));
    }

    #[test]
    fn exhaustion_blocks_attacks_until_energy_returns() {
        let mut g = corridor(10);
        g.player.energy = tuning::ATTACK_COST - 0.5;
        let mut out = Vec::new();
        g.apply(Command::Attack, &mut out);
        assert_eq!(out, [Event::Exhausted]);
        g.tick(0.5 / tuning::ENERGY_REGEN + 0.01, &mut out);
        out.clear();
        g.apply(Command::Attack, &mut out);
        assert_eq!(out, [Event::Swung { hit: None }]);
    }

    #[test]
    fn hitting_a_wind_up_staggers() {
        let mut g = corridor(2);
        let mut out = run(&mut g, DT * 2.0);
        assert!(g.enemies[0].is_winding_up());
        out.clear();
        g.apply(Command::Attack, &mut out);
        assert!(out.iter().any(|e| matches!(
            e,
            Event::EnemyHurt {
                staggered: true,
                ..
            }
        )));
        // The interrupted strike never lands.
        let hp = g.player.hp;
        let events = run(&mut g, enemy_tuning::STAGGER - 0.05);
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, Event::EnemyStruck { .. }))
        );
        assert_eq!(g.player.hp, hp);
    }

    #[test]
    fn death_stops_the_world() {
        let mut g = corridor(2);
        g.player.hp = 1;
        let events = run(&mut g, 3.0);
        assert!(events.contains(&Event::PlayerDied));
        assert!(!g.player.is_alive());
        let mut out = Vec::new();
        g.apply(Command::Step(Step::Forward), &mut out);
        g.tick(1.0, &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn kills_drop_gold_and_walking_over_it_takes_it() {
        let mut g = corridor(3);
        let mut out = Vec::new();
        g.player.facing = Facing::East;
        g.player.pos = GridPos::new(2, 1);
        kill_in_front(&mut g, &mut out);
        let dropped = out.iter().find_map(|e| match e {
            Event::GoldDropped { at, amount } => Some((*at, *amount)),
            _ => None,
        });
        let (at, amount) = dropped.expect("a kill drops gold");
        assert_eq!(at, GridPos::new(3, 1));
        assert!(amount > 0);
        assert_eq!(g.drops.len(), 1);
        assert_eq!((g.drops[0].pos, g.drops[0].gold), (at, amount));
        let items = g.drops[0].items;
        out.clear();
        g.apply(Command::Step(Step::Forward), &mut out);
        assert!(out.contains(&Event::GoldTaken { at, amount }));
        assert_eq!(g.player.gold, amount);
        // Any item in the pool came along too.
        assert_eq!(g.player.bag, items);
        assert!(g.drops.is_empty());
    }

    #[test]
    fn potions_drop_now_and_then_and_heal() {
        let mut g = corridor(3);
        let mut out = Vec::new();
        g.player.pos = GridPos::new(2, 1);
        for _ in 0..40 {
            g.enemies = alloc::vec![Enemy::of_kind(1, GridPos::new(3, 1), EnemyKind::Zombie)];
            kill_in_front(&mut g, &mut out);
        }
        assert!(
            g.drops[0].items.count(Item::HealthPotion) > 0,
            "40 kills, no potion"
        );
        g.player.bag.add(Item::HealthPotion);
        g.player.hp = 5;
        g.apply(Command::UseItem(Item::HealthPotion), &mut out);
        assert_eq!(g.player.hp, 15);
    }

    /// A corridor with enemies at the given x positions, the player at x=1
    /// facing east, full of Chi.
    fn lined_up(xs: &[i32]) -> Game {
        let mut g = corridor(9);
        g.enemies = xs
            .iter()
            .enumerate()
            .map(|(i, &x)| Enemy::of_kind(i as EnemyId, GridPos::new(x, 1), EnemyKind::DemonKnight))
            .collect();
        g.player.energy = g.player.max_energy;
        g
    }

    fn hits(out: &[Event]) -> Vec<(EnemyId, i32)> {
        out.iter()
            .filter_map(|e| match e {
                Event::EnemyHurt { id, damage, .. } => Some((*id, *damage)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn spark_flickers_three_to_five_times_at_the_nearest_ahead() {
        let mut counts = std::collections::HashSet::new();
        for seed in 0..40 {
            let mut g = lined_up(&[3, 4]);
            g.rng = Rng::new(seed);
            let mut out = Vec::new();
            g.apply(Command::Cast(Spell::Spark), &mut out);
            let h = hits(&out);
            assert!(h.iter().all(|&(id, d)| id == 0 && d == 1), "{h:?}");
            assert!((3..=5).contains(&h.len()));
            counts.insert(h.len());
            assert!((g.player.energy - (g.player.max_energy - Spell::Spark.cost())).abs() < 1e-4);
        }
        assert_eq!(counts.len(), 3, "3, 4 and 5 sparks all happen");
        // Out of range, nothing happens and nothing is spent.
        let mut g = lined_up(&[6]);
        let mut out = Vec::new();
        g.apply(Command::Cast(Spell::Spark), &mut out);
        assert_eq!(
            out,
            [Event::SpellFizzled {
                spell: Spell::Spark
            }]
        );
        assert_eq!(g.player.energy, g.player.max_energy);
    }

    #[test]
    fn the_bolt_goes_through_everything_in_line_and_stops_at_walls() {
        let mut g = lined_up(&[3, 5, 7, 9]);
        let mut out = Vec::new();
        g.apply(Command::Cast(Spell::LightningBolt), &mut out);
        // Range 6 from x=1 reaches x=7; x=9 is too far.
        let ids: Vec<_> = hits(&out).iter().map(|h| h.0).collect();
        assert_eq!(ids, [0, 1, 2]);
        assert!(out.contains(&Event::SpellCast {
            spell: Spell::LightningBolt,
            from: GridPos::new(1, 1),
            reach: GridPos::new(7, 1),
        }));
        // Again at once: still cooling down.
        out.clear();
        g.player.energy = g.player.max_energy;
        g.apply(Command::Cast(Spell::LightningBolt), &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn chain_lightning_arcs_from_foe_to_foe_and_weakens() {
        let mut g = lined_up(&[3, 5, 7, 9, 10]);
        let mut out = Vec::new();
        g.apply(Command::Cast(Spell::ChainLightning), &mut out);
        // Nearest first, then each next within 3, at most 4, 5-4-3-2.
        assert_eq!(hits(&out), [(0, 5), (1, 4), (2, 3), (3, 2)]);
        assert!((g.player.energy - (g.player.max_energy - 5.0)).abs() < 1e-4);
    }

    #[test]
    fn spells_need_chi_and_kills_by_spell_pay() {
        let mut g = lined_up(&[2]);
        g.player.energy = 1.0;
        let mut out = Vec::new();
        g.apply(Command::Cast(Spell::LightningBolt), &mut out);
        assert_eq!(out, [Event::Exhausted]);
        g.enemies[0].hp = 1;
        g.player.energy = g.player.max_energy;
        out.clear();
        g.apply(Command::Cast(Spell::LightningBolt), &mut out);
        assert!(out.iter().any(|e| matches!(e, Event::EnemyDied { .. })));
        assert!(out.iter().any(|e| matches!(e, Event::GoldDropped { .. })));
        assert!(out.iter().any(|e| matches!(e, Event::GainedXp { .. })));
    }

    /// Kill whatever is in front of the player, as many times as needed.
    fn kill_in_front(g: &mut Game, out: &mut Vec<Event>) {
        while g
            .enemies
            .iter()
            .any(|e| e.pos == g.player.pos + g.player.facing)
        {
            g.player.attack_cooldown = 0.0;
            g.player.energy = g.player.max_energy;
            g.apply(Command::Attack, out);
        }
    }

    #[test]
    fn kills_level_you_up_and_choices_wait_for_you() {
        let mut g = corridor(2);
        let mut out = Vec::new();
        g.depth = 10;
        // Enough kills for a few levels, without choosing anything.
        for _ in 0..8 {
            g.enemies = alloc::vec![Enemy::of_kind(
                1,
                GridPos::new(2, 1),
                EnemyKind::DemonKnight
            )];
            kill_in_front(&mut g, &mut out);
        }
        let levels = out
            .iter()
            .filter(|e| matches!(e, Event::LeveledUp { .. }))
            .count();
        assert!(levels >= 3, "{levels} levels");
        assert_eq!(g.player.level as usize, 1 + levels);
        assert_eq!(g.pending.len(), levels, "every level banks a choice");
        assert!(g.pending.iter().all(|o| o.len() == 3));

        // Choosing takes the ability and its effect.
        let before = g.player.max_hp;
        let offer = g.pending[0].clone();
        let pick = offer.iter().position(|&p| p == Perk::Vigor).unwrap_or(0);
        out.clear();
        g.apply(Command::Choose(pick as u8), &mut out);
        assert_eq!(out, [Event::PerkTaken { perk: offer[pick] }]);
        assert_eq!(g.player.perks.count(offer[pick]), 1);
        if offer[pick] == Perk::Vigor {
            assert_eq!(g.player.max_hp, before + 4);
        }
        assert_eq!(g.pending.len(), levels - 1);
    }

    #[test]
    fn level_36_is_the_top() {
        let mut g = corridor(2);
        let mut out = Vec::new();
        g.depth = 30;
        for _ in 0..400 {
            g.enemies = alloc::vec![Enemy::of_kind(
                1,
                GridPos::new(2, 1),
                EnemyKind::DemonKnight
            )];
            kill_in_front(&mut g, &mut out);
            while !g.pending.is_empty() {
                g.apply(Command::Choose(0), &mut out);
            }
        }
        assert_eq!(g.player.level, MAX_LEVEL);
        assert_eq!(g.player.xp, 0);
        let taken: u32 = g.player.perks.taken().map(|(_, n)| n as u32).sum();
        assert_eq!(taken, MAX_LEVEL - 1, "one ability per level gained");
        for (perk, n) in g.player.perks.taken() {
            assert!(n <= perk.max_stacks(), "{perk:?} {n}");
        }
    }

    #[test]
    fn abilities_change_the_rules() {
        let mut g = corridor(2);
        let mut out = Vec::new();
        for perk in [
            Perk::KeenEdge,
            Perk::LightBlade,
            Perk::QuickHands,
            Perk::IronSkin,
        ] {
            for _ in 0..3 {
                g.player.perks.add(perk);
            }
        }
        assert!((g.player.attack_cost() - 1.8).abs() < 1e-4);
        assert!((g.player.attack_cooldown_secs() - 0.35).abs() < 1e-4);
        // Keen Edge: every hit does at least 2 + 3.
        g.enemies = alloc::vec![Enemy::of_kind(
            1,
            GridPos::new(2, 1),
            EnemyKind::DemonKnight
        )];
        g.apply(Command::Attack, &mut out);
        let dealt = out.iter().find_map(|e| match e {
            Event::EnemyHurt { damage, .. } => Some(*damage),
            _ => None,
        });
        assert!(dealt.is_some_and(|d| d >= 5), "{dealt:?}");
        // Iron Skin: a demon knight's 5..8 lands as 2..5.
        g.enemies[0].state = EnemyState::WindUp { left: 0.01 };
        let hp = g.player.hp;
        let events = run(&mut g, 0.05);
        let struck = events.iter().find_map(|e| match e {
            Event::EnemyStruck { damage, .. } => Some(*damage),
            _ => None,
        });
        assert!(struck.is_some_and(|d| (2..=5).contains(&d)), "{struck:?}");
        assert_eq!(g.player.hp, hp - struck.unwrap());
    }

    #[test]
    fn randomized_runs_bring_stages_and_crowds() {
        let mut g = Game::new(5);
        for _ in 0..13 {
            g.player.pos = g.floor.stairs;
            g.apply(Command::Descend, &mut Vec::new());
        }
        assert_eq!(g.depth, 14);
        let stage = g.floor.stage.expect("generated floors belong to a stage");
        assert_eq!(stage.number, 4);
        assert!(g.enemies.len() >= 2, "deeper floors hold more enemies");
        for e in &g.enemies {
            assert!(stage.theme.kinds().contains(&e.kind) || e.kind.first_depth() <= 2);
        }
    }

    #[test]
    fn an_arrow_flies_before_it_strikes() {
        let mut g = corridor(5);
        let hp = g.enemies[0].hp;
        let mut out = Vec::new();
        g.apply(Command::Shoot, &mut out);
        assert!(matches!(
            out[0],
            Event::Launched {
                missile: Missile::Arrow,
                ..
            }
        ));
        assert_eq!(g.player.arrows, arrow_tuning::START_ARROWS - 1);
        assert!(g.enemies[0].hp == hp && g.flights.len() == 1, "not yet");
        let out = run(&mut g, 0.6);
        assert!(
            out.iter()
                .any(|e| matches!(e, Event::MissileHit { enemy: 7, .. }))
        );
        assert!(g.enemies.first().is_none_or(|e| e.hp < hp));
        assert!(g.flights.is_empty());
    }

    #[test]
    fn a_miss_falls_at_the_wall_and_can_be_picked_up() {
        let mut g = corridor(5);
        g.enemies.clear();
        g.player.pos = GridPos::new(5, 1);
        let mut out = Vec::new();
        g.apply(Command::Shoot, &mut out);
        let out = run(&mut g, 1.0);
        let landed = out.iter().find_map(|e| match e {
            Event::ArrowLanded { at, wall, kept, .. } => Some((*at, *wall, *kept)),
            _ => None,
        });
        let (at, wall, kept) = landed.expect("it came down");
        assert_eq!((at, wall), (GridPos::new(10, 1), true));
        let before = g.player.arrows;
        g.player.pos = GridPos::new(9, 1);
        let mut out = Vec::new();
        g.apply(Command::Step(Step::Forward), &mut out);
        let taken = u32::from(kept);
        assert_eq!(g.player.arrows, before + taken);
        assert!(g.drops.is_empty());
    }

    #[test]
    fn a_point_blank_miss_comes_straight_back() {
        let mut g = corridor(5);
        g.enemies.clear();
        g.player.facing = Facing::North;
        let mut out = Vec::new();
        g.apply(Command::Shoot, &mut out);
        let out = run(&mut g, 0.3);
        let kept = out
            .iter()
            .any(|e| matches!(e, Event::ArrowLanded { kept: true, .. }));
        let expect = arrow_tuning::START_ARROWS - u32::from(!kept);
        assert_eq!(g.player.arrows, expect);
        assert!(g.drops.is_empty(), "nothing left under your feet");
    }

    #[test]
    fn arrows_run_out() {
        let mut g = corridor(5);
        g.player.arrows = 0;
        let mut out = Vec::new();
        g.apply(Command::Shoot, &mut out);
        assert_eq!(out, [Event::OutOfArrows]);
        assert!(g.flights.is_empty());
    }

    #[test]
    fn a_spent_arrow_falls_at_the_end_of_its_range() {
        let mut g = Game::new(0);
        let mut map = Map::filled(20, 3, Tile::Wall);
        for x in 1..19 {
            map.set(GridPos::new(x, 1), Tile::Floor);
        }
        g.floor.map = map;
        g.player = Player::new(GridPos::new(1, 1), Facing::East);
        g.enemies.clear();
        let mut out = Vec::new();
        g.apply(Command::Shoot, &mut out);
        let out = run(&mut g, 1.5);
        let at = out.iter().find_map(|e| match e {
            Event::ArrowLanded {
                at, wall: false, ..
            } => Some(*at),
            _ => None,
        });
        assert_eq!(at, Some(GridPos::new(1 + arrow_tuning::RANGE as i32, 1)));
    }

    #[test]
    fn the_psionic_blade_lasts_five_swings_and_costs_no_chi() {
        let mut g = corridor(2);
        g.enemies[0].hp = 1000;
        let mut out = Vec::new();
        g.apply(Command::Cast(Spell::PsionicBlade), &mut out);
        assert_eq!(g.player.blade, spell_tuning::BLADE_SWINGS);
        let chi = g.player.energy;
        let mut damage = Vec::new();
        for _ in 0..spell_tuning::BLADE_SWINGS {
            let mut out = Vec::new();
            g.player.attack_cooldown = 0.0;
            g.apply(Command::Attack, &mut out);
            damage.extend(out.iter().filter_map(|e| match e {
                Event::EnemyHurt { damage, .. } => Some(*damage),
                _ => None,
            }));
            let last = g.player.blade == 0;
            assert_eq!(out.contains(&Event::BladeFaded), last);
        }
        assert_eq!(g.player.energy, chi, "free while it lasts");
        assert!(damage.iter().all(|&d| d >= spell_tuning::BLADE_DAMAGE_MIN));
        // Then the sword again, at the sword's price.
        let mut out = Vec::new();
        g.player.attack_cooldown = 0.0;
        g.apply(Command::Attack, &mut out);
        assert!(g.player.energy < chi);
    }

    #[test]
    fn a_hammer_strikes_and_comes_home() {
        let mut g = corridor(5);
        g.enemies[0].hp = 1000;
        let mut out = Vec::new();
        g.apply(Command::Cast(Spell::MagicHammer), &mut out);
        assert_eq!(g.hammers_out(), 1);
        let out = run(&mut g, 2.0);
        let hit = out.iter().position(|e| {
            matches!(
                e,
                Event::MissileHit {
                    missile: Missile::Hammer,
                    ..
                }
            )
        });
        let turned = out
            .iter()
            .position(|e| matches!(e, Event::HammerTurned { .. }));
        let caught = out
            .iter()
            .position(|e| matches!(e, Event::HammerCaught { .. }));
        assert!(
            hit < turned && turned < caught && caught.is_some(),
            "{out:?}"
        );
        assert_eq!(g.hammers_out(), 0);
    }

    #[test]
    fn two_hammers_at_once_thwack_thwack() {
        let mut g = corridor(6);
        g.enemies[0].hp = 1000;
        let mut out = Vec::new();
        g.apply(Command::Cast(Spell::MagicHammer), &mut out);
        let _ = run(&mut g, 0.3);
        g.apply(Command::Cast(Spell::MagicHammer), &mut out);
        assert_eq!(g.hammers_out(), 2);
        // A third waits until one is back.
        let _ = run(&mut g, 0.3);
        g.apply(Command::Cast(Spell::MagicHammer), &mut out);
        assert_eq!(g.hammers_out(), 2);
        let out = run(&mut g, 2.5);
        let thwacks = out
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    Event::MissileHit {
                        missile: Missile::Hammer,
                        ..
                    }
                )
            })
            .count();
        assert!(thwacks >= 1, "{out:?}");
        assert_eq!(g.hammers_out(), 0);
    }

    #[test]
    fn a_spark_barrage_is_a_stream_of_little_hits() {
        let mut g = corridor(5);
        g.enemies[0].hp = 1000;
        g.player.energy = g.player.max_energy;
        let mut out = Vec::new();
        // Held down: cast every frame; the cooldown sets the rate.
        for _ in 0..(1.0 / DT) as usize {
            g.apply(Command::Cast(Spell::SparkBarrage), &mut out);
            g.tick(DT, &mut out);
        }
        let sparks = out
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    Event::Launched {
                        missile: Missile::Spark,
                        ..
                    }
                )
            })
            .count();
        let hits = out
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    Event::MissileHit {
                        missile: Missile::Spark,
                        ..
                    }
                )
            })
            .count();
        assert!((8..=12).contains(&sparks), "{sparks} sparks in a second");
        assert!(hits >= sparks - 2, "{hits} of {sparks} hit");
    }
}

#[cfg(test)]
mod stairs_tests {
    use super::*;

    /// Walk the shortest path to the stairs, one step at a time.
    fn walk_to_stairs(g: &mut Game) {
        let mut out = Vec::new();
        g.enemies.clear(); // no interruptions
        let dist = g.floor.map.distances(g.floor.stairs);
        while !g.on_stairs() {
            let here = dist[g.floor.map.index(g.player.pos)].unwrap();
            let dir = Facing::ALL
                .into_iter()
                .find(|&d| dist[g.floor.map.index(g.player.pos + d)].is_some_and(|n| n < here))
                .unwrap();
            while g.player.facing != dir {
                g.apply(Command::Turn(Turn::Right), &mut out);
            }
            g.apply(Command::Step(Step::Forward), &mut out);
        }
    }

    #[test]
    fn descend_only_from_the_stairs() {
        let mut g = Game::new(11);
        let mut out = Vec::new();
        g.apply(Command::Descend, &mut out);
        assert!(out.is_empty());
        assert_eq!(g.depth, 1);
        walk_to_stairs(&mut g);
        g.apply(Command::Descend, &mut out);
        assert_eq!(out, [Event::Descended { depth: 2 }]);
        assert_eq!(g.depth, 2);
        assert_eq!(g.player.pos, g.floor.start);
        assert_eq!(g.enemies.len(), 1);
    }

    #[test]
    fn floors_are_endless_and_reproducible() {
        let mut a = Game::new(5);
        let mut b = Game::new(5);
        for depth in 2..=100 {
            walk_to_stairs(&mut a);
            walk_to_stairs(&mut b);
            let mut out = Vec::new();
            a.apply(Command::Descend, &mut out);
            b.apply(Command::Descend, &mut out);
            assert_eq!(a.depth, depth);
            assert_eq!(a.floor.stairs, b.floor.stairs);
            assert_eq!(a.floor.start, b.floor.start);
        }
    }

    #[test]
    fn descending_heals_a_little() {
        let mut g = Game::new(2);
        g.player.hp = 3;
        walk_to_stairs(&mut g);
        g.apply(Command::Descend, &mut Vec::new());
        assert_eq!(g.player.hp, 3 + tuning::DESCEND_HEAL);
    }

    #[test]
    fn deeper_floors_bring_new_kinds() {
        let kinds_at = |depth: u32| -> std::collections::HashSet<EnemyKind> {
            (0..400u64)
                .map(|r| kind_for(Place::Dungeon, depth, mix64(r)))
                .collect()
        };
        assert_eq!(kind_for(Place::Ruins, 1, 0), EnemyKind::Undead);
        let shallow = kinds_at(2);
        assert!(shallow.contains(&EnemyKind::Zombie));
        assert!(!shallow.contains(&EnemyKind::DemonKnight));
        assert_eq!(kinds_at(9).len(), EnemyKind::ALL.len());
    }
}
