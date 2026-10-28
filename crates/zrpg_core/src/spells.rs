//! Spells: lightning, in three shapes. Each costs Chi and has its own
//! cooldown; the casting itself (who is hit, in what order) is in
//! `game.rs`. Damage grows a little with level.

/// The spells, in the order of their keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Spell {
    /// A faint flurry of small hits on the nearest foe ahead.
    Spark,
    /// Strikes the nearest foe in sight, then arcs on to others nearby.
    ChainLightning,
    /// A straight bolt down your line of sight, through everything in it.
    LightningBolt,
}

pub mod tuning {
    /// Spark: reach (cells ahead), hits per cast and damage per hit.
    pub const SPARK_RANGE: i32 = 3;
    pub const SPARK_HITS_MIN: i32 = 3;
    pub const SPARK_HITS_MAX: i32 = 5;
    pub const SPARK_DAMAGE: i32 = 1;
    /// Chain lightning: reach to the first foe and between foes, how many
    /// it can strike, and its damage to each in turn.
    pub const CHAIN_RANGE: i32 = 4;
    pub const CHAIN_JUMP: i32 = 3;
    pub const CHAIN_TARGETS: usize = 4;
    pub const CHAIN_DAMAGE: [i32; 4] = [5, 4, 3, 2];
    /// Lightning bolt: reach and damage to everything in the line.
    pub const BOLT_RANGE: i32 = 6;
    pub const BOLT_DAMAGE: i32 = 4;
    /// Levels per extra point of spell damage.
    pub const LEVELS_PER_BONUS: u32 = 6;
}

impl Spell {
    pub const ALL: [Spell; 3] = [Spell::Spark, Spell::ChainLightning, Spell::LightningBolt];

    pub fn index(self) -> usize {
        Spell::ALL.iter().position(|&s| s == self).unwrap_or(0)
    }

    pub const fn name(self) -> &'static str {
        match self {
            Spell::Spark => "SPARK",
            Spell::ChainLightning => "CHAIN LIGHTNING",
            Spell::LightningBolt => "LIGHTNING BOLT",
        }
    }

    /// Chi per cast.
    pub const fn cost(self) -> f32 {
        match self {
            Spell::Spark => 2.0,
            Spell::ChainLightning => 5.0,
            Spell::LightningBolt => 4.0,
        }
    }

    /// Seconds before it can be cast again.
    pub const fn cooldown(self) -> f32 {
        match self {
            Spell::Spark => 0.7,
            Spell::ChainLightning => 1.6,
            Spell::LightningBolt => 1.2,
        }
    }
}

/// Extra spell damage at `level`.
pub fn level_bonus(level: u32) -> i32 {
    ((level.saturating_sub(1)) / tuning::LEVELS_PER_BONUS) as i32
}
