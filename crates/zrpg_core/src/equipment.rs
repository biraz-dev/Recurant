//! Equipment: what you wear and hold, one piece per slot. For now you start
//! with the sword in your right hand, the torch in your left, and a bow and
//! quiver on your back; nothing else drops yet. The slots are ready for
//! more.
//!
//! Gear doesn't change yet, so it isn't saved: a resumed run starts with
//! the same kit.

/// Where a piece of gear goes, in the order the equipment screen lists
/// them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Slot {
    Helmet,
    Necklace,
    Armor,
    RightHand,
    LeftHand,
    Gloves,
    Belt,
    RingRight,
    RingLeft,
    Bow,
    Quiver,
}

impl Slot {
    pub const ALL: [Slot; 11] = [
        Slot::Helmet,
        Slot::Necklace,
        Slot::Armor,
        Slot::RightHand,
        Slot::LeftHand,
        Slot::Gloves,
        Slot::Belt,
        Slot::RingRight,
        Slot::RingLeft,
        Slot::Bow,
        Slot::Quiver,
    ];

    pub fn index(self) -> usize {
        Slot::ALL.iter().position(|&s| s == self).unwrap_or(0)
    }

    pub const fn name(self) -> &'static str {
        match self {
            Slot::Helmet => "HELMET",
            Slot::Necklace => "NECKLACE",
            Slot::Armor => "ARMOR",
            Slot::RightHand => "RIGHT HAND",
            Slot::LeftHand => "LEFT HAND",
            Slot::Gloves => "GLOVES",
            Slot::Belt => "BELT",
            Slot::RingRight => "RING",
            Slot::RingLeft => "RING",
            Slot::Bow => "BOW",
            Slot::Quiver => "QUIVER",
        }
    }
}

/// A piece of gear.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Gear {
    Sword,
    Torch,
    HuntingBow,
    Quiver,
}

impl Gear {
    pub const ALL: [Gear; 4] = [Gear::Sword, Gear::Torch, Gear::HuntingBow, Gear::Quiver];

    pub fn index(self) -> usize {
        Gear::ALL.iter().position(|&g| g == self).unwrap_or(0)
    }

    pub const fn name(self) -> &'static str {
        match self {
            Gear::Sword => "WORN SWORD",
            Gear::Torch => "TORCH",
            Gear::HuntingBow => "HUNTING BOW",
            Gear::Quiver => "QUIVER",
        }
    }

    pub const fn description(self) -> &'static str {
        match self {
            Gear::Sword => "Nicked, but it holds an edge.",
            Gear::Torch => "Keeps the dark a few steps back.",
            Gear::HuntingBow => "Ash and gut. Reaches eight cells down a straight line.",
            Gear::Quiver => "Holds up to sixty arrows.",
        }
    }

    /// The slot it goes in.
    pub const fn slot(self) -> Slot {
        match self {
            Gear::Sword => Slot::RightHand,
            Gear::Torch => Slot::LeftHand,
            Gear::HuntingBow => Slot::Bow,
            Gear::Quiver => Slot::Quiver,
        }
    }
}

/// What is in each slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Equipment {
    slots: [Option<Gear>; Slot::ALL.len()],
}

impl Equipment {
    /// Nothing at all.
    pub const EMPTY: Equipment = Equipment {
        slots: [None; Slot::ALL.len()],
    };

    /// What every run starts with: sword, torch, bow and quiver.
    pub fn starting_kit() -> Equipment {
        let mut kit = Equipment::EMPTY;
        for gear in Gear::ALL {
            kit.wear(gear);
        }
        kit
    }

    pub fn get(&self, slot: Slot) -> Option<Gear> {
        self.slots[slot.index()]
    }

    /// Put `gear` in its slot; returns what was there.
    pub fn wear(&mut self, gear: Gear) -> Option<Gear> {
        self.slots[gear.slot().index()].replace(gear)
    }

    /// Empty a slot; returns what was there.
    pub fn take_off(&mut self, slot: Slot) -> Option<Gear> {
        self.slots[slot.index()].take()
    }
}

impl Default for Equipment {
    fn default() -> Self {
        Equipment::starting_kit()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn you_start_with_sword_torch_and_bow() {
        let kit = Equipment::starting_kit();
        assert_eq!(kit.get(Slot::RightHand), Some(Gear::Sword));
        assert_eq!(kit.get(Slot::LeftHand), Some(Gear::Torch));
        assert_eq!(kit.get(Slot::Bow), Some(Gear::HuntingBow));
        assert_eq!(kit.get(Slot::Quiver), Some(Gear::Quiver));
        let worn = Slot::ALL.iter().filter(|&&s| kit.get(s).is_some()).count();
        assert_eq!(worn, 4);
    }

    #[test]
    fn wearing_swaps_out_the_old_piece() {
        let mut kit = Equipment::EMPTY;
        assert_eq!(kit.wear(Gear::Torch), None);
        assert_eq!(kit.wear(Gear::Torch), Some(Gear::Torch));
        assert_eq!(kit.take_off(Slot::LeftHand), Some(Gear::Torch));
        assert_eq!(kit.get(Slot::LeftHand), None);
    }
}
