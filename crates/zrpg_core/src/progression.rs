//! Levels and abilities. Kills give XP; each level (up to 36) offers three
//! random abilities, and you take one. Every ability is a rule change in
//! `game.rs`, and most stack a few times.
//!
//! Choices wait until they are made: in auto mode the bot never picks, so
//! level-ups bank up for you to spend when you take control.

use alloc::vec::Vec;

use crate::enemy::EnemyKind;
use crate::rng::Rng;

pub const MAX_LEVEL: u32 = 36;

/// XP needed to go from `level` to the next.
pub const fn xp_to_next(level: u32) -> u32 {
    12 + 8 * level
}

/// XP for a kill: tougher creatures and deeper floors are worth more.
pub fn xp_for_kill(kind: EnemyKind, depth: u32) -> u32 {
    kind.stats().max_hp as u32 + depth
}

/// The abilities.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Perk {
    Vigor,
    Stamina,
    SecondBreath,
    KeenEdge,
    LightBlade,
    QuickHands,
    IronSkin,
    Evasion,
    CrushingBlow,
    Bloodthirst,
    Regeneration,
    Scholar,
    Pilgrim,
    Concussion,
}

impl Perk {
    pub const ALL: [Perk; 14] = [
        Perk::Vigor,
        Perk::Stamina,
        Perk::SecondBreath,
        Perk::KeenEdge,
        Perk::LightBlade,
        Perk::QuickHands,
        Perk::IronSkin,
        Perk::Evasion,
        Perk::CrushingBlow,
        Perk::Bloodthirst,
        Perk::Regeneration,
        Perk::Scholar,
        Perk::Pilgrim,
        Perk::Concussion,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Perk::Vigor => "VIGOR",
            Perk::Stamina => "STAMINA",
            Perk::SecondBreath => "SECOND BREATH",
            Perk::KeenEdge => "KEEN EDGE",
            Perk::LightBlade => "LIGHT BLADE",
            Perk::QuickHands => "QUICK HANDS",
            Perk::IronSkin => "IRON SKIN",
            Perk::Evasion => "EVASION",
            Perk::CrushingBlow => "CRUSHING BLOW",
            Perk::Bloodthirst => "BLOODTHIRST",
            Perk::Regeneration => "REGENERATION",
            Perk::Scholar => "SCHOLAR",
            Perk::Pilgrim => "PILGRIM",
            Perk::Concussion => "CONCUSSION",
        }
    }

    /// One short line, for the level-up picker.
    pub const fn description(self) -> &'static str {
        match self {
            Perk::Vigor => "+4 max Health",
            Perk::Stamina => "+2 max Energy",
            Perk::SecondBreath => "Energy returns 25% faster",
            Perk::KeenEdge => "+1 damage per hit",
            Perk::LightBlade => "Swings cost less Energy",
            Perk::QuickHands => "Swing again sooner",
            Perk::IronSkin => "Take 1 less damage per blow",
            Perk::Evasion => "6% chance to dodge a blow",
            Perk::CrushingBlow => "7% chance of double damage",
            Perk::Bloodthirst => "Heal 1 Health per kill",
            Perk::Regeneration => "Slowly heal over time",
            Perk::Scholar => "+15% XP",
            Perk::Pilgrim => "Heal 3 more on each new floor",
            Perk::Concussion => "Staggered foes reel longer",
        }
    }

    /// How many times it can be taken.
    pub const fn max_stacks(self) -> u8 {
        match self {
            Perk::Vigor | Perk::KeenEdge | Perk::Evasion | Perk::CrushingBlow => 5,
            Perk::Stamina
            | Perk::SecondBreath
            | Perk::LightBlade
            | Perk::QuickHands
            | Perk::Bloodthirst => 4,
            Perk::IronSkin
            | Perk::Regeneration
            | Perk::Scholar
            | Perk::Pilgrim
            | Perk::Concussion => 3,
        }
    }

    pub fn index(self) -> usize {
        Perk::ALL.iter().position(|&p| p == self).unwrap_or(0)
    }
}

/// How many times each ability has been taken.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Perks {
    counts: [u8; Perk::ALL.len()],
}

impl Perks {
    pub fn count(&self, perk: Perk) -> u8 {
        self.counts[perk.index()]
    }

    pub fn can_take(&self, perk: Perk) -> bool {
        self.count(perk) < perk.max_stacks()
    }

    pub(crate) fn add(&mut self, perk: Perk) {
        let i = perk.index();
        self.counts[i] = (self.counts[i] + 1).min(perk.max_stacks());
    }

    /// Taken abilities and their counts, in list order.
    pub fn taken(&self) -> impl Iterator<Item = (Perk, u8)> + '_ {
        Perk::ALL
            .into_iter()
            .map(|p| (p, self.count(p)))
            .filter(|&(_, n)| n > 0)
    }

    pub(crate) fn from_counts(counts: [u8; Perk::ALL.len()]) -> Self {
        let mut p = Perks { counts };
        for perk in Perk::ALL {
            let i = perk.index();
            p.counts[i] = p.counts[i].min(perk.max_stacks());
        }
        p
    }
}

/// Three different abilities you could take next (fewer only if nearly
/// everything is maxed out).
pub fn roll_offer(rng: &mut Rng, perks: &Perks) -> Vec<Perk> {
    let mut pool: Vec<Perk> = Perk::ALL
        .into_iter()
        .filter(|&p| perks.can_take(p))
        .collect();
    let mut offer = Vec::new();
    while offer.len() < 3 && !pool.is_empty() {
        let i = rng.range(0, pool.len() as i32 - 1) as usize;
        offer.push(pool.swap_remove(i));
    }
    offer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_36_is_a_long_road() {
        let total: u32 = (1..MAX_LEVEL).map(xp_to_next).sum();
        assert_eq!(total, 5460);
        assert_eq!(xp_to_next(1), 20);
        // A floor-2 zombie is worth 10; a floor-20 demon knight 36.
        assert_eq!(xp_for_kill(EnemyKind::Zombie, 2), 10);
        assert_eq!(xp_for_kill(EnemyKind::DemonKnight, 20), 36);
    }

    #[test]
    fn there_is_always_enough_to_offer() {
        // Every level up to 36 can offer three choices.
        let room: u32 = Perk::ALL.iter().map(|p| p.max_stacks() as u32).sum();
        assert!(room >= MAX_LEVEL - 1 + 2, "{room}");
        let mut rng = Rng::new(3);
        let mut perks = Perks::default();
        for _ in 1..MAX_LEVEL {
            let offer = roll_offer(&mut rng, &perks);
            assert_eq!(offer.len(), 3);
            assert!(offer[0] != offer[1] && offer[1] != offer[2] && offer[0] != offer[2]);
            assert!(offer.iter().all(|&p| perks.can_take(p)));
            perks.add(offer[0]);
        }
    }
}
