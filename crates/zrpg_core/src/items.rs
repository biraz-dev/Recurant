//! Items: what the dead sometimes leave besides gold, carried in a small bag
//! and used one at a time. For now there is one, the Health Potion; the
//! bag, pools and belt are built for more.

use crate::enemy::EnemyKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Item {
    /// Heals 10.
    HealthPotion,
}

/// At most this many of each item fit in the bag.
pub const MAX_CARRIED: u8 = 9;

pub mod tuning {
    pub const POTION_HEAL: i32 = 10;
    /// Percent chance any kill leaves a potion.
    pub const POTION_DROP_PERCENT: u32 = 15;
}

impl Item {
    pub const ALL: [Item; 1] = [Item::HealthPotion];

    pub fn index(self) -> usize {
        Item::ALL.iter().position(|&i| i == self).unwrap_or(0)
    }

    pub const fn name(self) -> &'static str {
        match self {
            Item::HealthPotion => "HEALTH POTION",
        }
    }

    pub const fn description(self) -> &'static str {
        match self {
            Item::HealthPotion => "Heals 10 Health",
        }
    }

    /// The chance, in percent, that a kill of `kind` leaves this item.
    pub const fn drop_percent(self, _kind: EnemyKind) -> u32 {
        match self {
            Item::HealthPotion => tuning::POTION_DROP_PERCENT,
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
    fn the_bag_has_limits() {
        let mut bag = Bag::default();
        assert!(bag.is_empty() && !bag.take(Item::HealthPotion));
        for _ in 0..MAX_CARRIED {
            assert!(bag.add(Item::HealthPotion));
        }
        assert!(!bag.add(Item::HealthPotion), "full");
        assert!(bag.take(Item::HealthPotion));
        assert_eq!(
            bag.items().collect::<Vec<_>>(),
            [(Item::HealthPotion, MAX_CARRIED - 1)]
        );
    }
}
