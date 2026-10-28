//! Items: what the dead leave behind besides gold. Each creature family has
//! its own, dropped now and then in the pool where it vanished; walking over
//! the pool takes everything. Items are carried in a small bag and used one
//! at a time.

use crate::enemy::EnemyKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Item {
    /// Heals 4.
    GraveMoss,
    /// Heals 8.
    LinenBandage,
    /// Refills Chi.
    EmberShard,
    /// The next blow at you passes through.
    Ectoplasm,
    /// 20 XP.
    BoneCharm,
    /// Your next three hits do double damage.
    DemonSigil,
}

/// At most this many of each item fit in the bag.
pub const MAX_CARRIED: u8 = 9;

pub mod tuning {
    pub const GRAVE_MOSS_HEAL: i32 = 4;
    pub const BANDAGE_HEAL: i32 = 8;
    pub const BONE_CHARM_XP: u32 = 20;
    /// Hits empowered by one Demon Sigil.
    pub const SIGIL_HITS: u8 = 3;
    /// Most blows Ectoplasm can ward off at once.
    pub const MAX_WARD: u8 = 3;
    /// Percent chance a kill drops its family's item; more for the strongest.
    pub const DROP_PERCENT: u32 = 35;
    pub const RARE_DROP_PERCENT: u32 = 60;
}

impl Item {
    pub const ALL: [Item; 6] = [
        Item::GraveMoss,
        Item::LinenBandage,
        Item::EmberShard,
        Item::Ectoplasm,
        Item::BoneCharm,
        Item::DemonSigil,
    ];

    pub fn index(self) -> usize {
        Item::ALL.iter().position(|&i| i == self).unwrap_or(0)
    }

    pub const fn name(self) -> &'static str {
        match self {
            Item::GraveMoss => "GRAVE MOSS",
            Item::LinenBandage => "LINEN BANDAGE",
            Item::EmberShard => "EMBER SHARD",
            Item::Ectoplasm => "ECTOPLASM",
            Item::BoneCharm => "BONE CHARM",
            Item::DemonSigil => "DEMON SIGIL",
        }
    }

    pub const fn description(self) -> &'static str {
        match self {
            Item::GraveMoss => "Heals 4 Health",
            Item::LinenBandage => "Heals 8 Health",
            Item::EmberShard => "Refills Chi",
            Item::Ectoplasm => "The next blow passes through you",
            Item::BoneCharm => "Grants 20 XP",
            Item::DemonSigil => "Your next 3 hits do double damage",
        }
    }

    /// What a creature leaves behind, if anything.
    pub const fn dropped_by(kind: EnemyKind) -> Item {
        match kind {
            EnemyKind::Zombie | EnemyKind::Undead => Item::GraveMoss,
            EnemyKind::Mummy | EnemyKind::RoyalMummy => Item::LinenBandage,
            EnemyKind::EmberSkeleton | EnemyKind::InnerDemon => Item::EmberShard,
            EnemyKind::Wraith => Item::Ectoplasm,
            EnemyKind::SkeletonWarrior | EnemyKind::SkeletonKnight => Item::BoneCharm,
            EnemyKind::DemonKnight => Item::DemonSigil,
        }
    }

    /// The chance, in percent, that a kill of `kind` drops its item.
    pub const fn drop_percent(kind: EnemyKind) -> u32 {
        match kind {
            EnemyKind::RoyalMummy | EnemyKind::DemonKnight => tuning::RARE_DROP_PERCENT,
            _ => tuning::DROP_PERCENT,
        }
    }
}

/// How many of each item: in the bag, or lying in a pool.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Bag {
    counts: [u8; Item::ALL.len()],
}

impl Bag {
    pub fn count(&self, item: Item) -> u8 {
        self.counts[item.index()]
    }

    pub fn is_empty(&self) -> bool {
        self.counts.iter().all(|&n| n == 0)
    }

    /// Add one; returns false if there is no room.
    pub fn add(&mut self, item: Item) -> bool {
        let n = &mut self.counts[item.index()];
        if *n >= MAX_CARRIED {
            return false;
        }
        *n += 1;
        true
    }

    /// Take one out; returns false if there was none.
    pub fn take(&mut self, item: Item) -> bool {
        let n = &mut self.counts[item.index()];
        if *n == 0 {
            return false;
        }
        *n -= 1;
        true
    }

    /// Items present and their counts, in list order.
    pub fn items(&self) -> impl Iterator<Item = (Item, u8)> + '_ {
        Item::ALL
            .into_iter()
            .map(|i| (i, self.count(i)))
            .filter(|&(_, n)| n > 0)
    }

    pub(crate) fn counts(&self) -> [u8; Item::ALL.len()] {
        self.counts
    }

    pub(crate) fn from_counts(counts: [u8; Item::ALL.len()]) -> Bag {
        Bag {
            counts: counts.map(|n| n.min(MAX_CARRIED)),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use super::*;

    #[test]
    fn every_creature_has_an_item_and_the_bag_has_limits() {
        for kind in EnemyKind::ALL {
            let _ = Item::dropped_by(kind);
            assert!((1..=100).contains(&Item::drop_percent(kind)));
        }
        let mut bag = Bag::default();
        assert!(bag.is_empty() && !bag.take(Item::BoneCharm));
        for _ in 0..MAX_CARRIED {
            assert!(bag.add(Item::BoneCharm));
        }
        assert!(!bag.add(Item::BoneCharm), "full");
        assert_eq!(bag.count(Item::BoneCharm), MAX_CARRIED);
        assert!(bag.take(Item::BoneCharm));
        assert_eq!(
            bag.items().collect::<Vec<_>>(),
            [(Item::BoneCharm, MAX_CARRIED - 1)]
        );
    }
}
