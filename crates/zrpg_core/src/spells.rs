//! Spells: lightning in three shapes, the psionic blade, the magic hammer
//! (after Dungeon Hack) and the spark barrage (after Lands of Lore). Each
//! costs Chi and has its own cooldown; the casting itself (who is hit, in
//! what order) is in `game.rs`, and the hammer and the sparks fly like
//! arrows (`archery.rs`). Damage grows a little with level.

/// The spells, in the order of their keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Spell {
    /// A faint flurry of small hits on the nearest foe ahead.
    Spark,
    /// Strikes the nearest foe in sight, then arcs on to others nearby.
    ChainLightning,
    /// A straight bolt down your line of sight, through everything in it.
    LightningBolt,
    /// Conjures a blade of thought in place of your sword: it hits far
    /// harder and swings without Chi, for a few swings, then comes apart.
    PsionicBlade,
    /// Throws a magic hammer down the line: it strikes the first foe and
    /// flies back to your hand. Two can be in the air at once.
    MagicHammer,
    /// Hold to hose a stream of little sparks down the line, each one a
    /// missile of its own.
    SparkBarrage,
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
    /// Psionic blade: swings it lasts (hit or miss), and its damage per hit
    /// (with Keen Edge and the level bonus on top).
    pub const BLADE_SWINGS: u8 = 5;
    pub const BLADE_DAMAGE_MIN: i32 = 6;
    pub const BLADE_DAMAGE_MAX: i32 = 9;
    /// Magic hammer: how many can be in the air, how fast and far they fly
    /// (cells per second, cells), and damage per blow.
    pub const HAMMERS: usize = 2;
    pub const HAMMER_SPEED: f32 = 9.0;
    pub const HAMMER_RANGE: u32 = 6;
    pub const HAMMER_DAMAGE_MIN: i32 = 5;
    pub const HAMMER_DAMAGE_MAX: i32 = 7;
    /// Spark barrage: each spark's speed, reach and damage (the barrage's
    /// rate of fire is its cooldown, its price per spark its cost).
    pub const BARRAGE_SPEED: f32 = 18.0;
    pub const BARRAGE_RANGE: u32 = 7;
    pub const BARRAGE_DAMAGE: i32 = 1;
    /// Levels per extra point of spell damage.
    pub const LEVELS_PER_BONUS: u32 = 6;
}

impl Spell {
    pub const ALL: [Spell; 6] = [
        Spell::Spark,
        Spell::ChainLightning,
        Spell::LightningBolt,
        Spell::PsionicBlade,
        Spell::MagicHammer,
        Spell::SparkBarrage,
    ];

    pub fn index(self) -> usize {
        Spell::ALL.iter().position(|&s| s == self).unwrap_or(0)
    }

    pub const fn name(self) -> &'static str {
        match self {
            Spell::Spark => "SPARK",
            Spell::ChainLightning => "CHAIN LIGHTNING",
            Spell::LightningBolt => "LIGHTNING BOLT",
            Spell::PsionicBlade => "PSIONIC BLADE",
            Spell::MagicHammer => "MAGIC HAMMER",
            Spell::SparkBarrage => "SPARK BARRAGE",
        }
    }

    /// Chi per cast.
    pub const fn cost(self) -> f32 {
        match self {
            Spell::Spark => 2.0,
            Spell::ChainLightning => 5.0,
            Spell::LightningBolt => 4.0,
            Spell::PsionicBlade => 6.0,
            Spell::MagicHammer => 3.0,
            Spell::SparkBarrage => 0.5,
        }
    }

    /// Seconds before it can be cast again.
    pub const fn cooldown(self) -> f32 {
        match self {
            Spell::Spark => 0.7,
            Spell::ChainLightning => 1.6,
            Spell::LightningBolt => 1.2,
            Spell::PsionicBlade => 1.5,
            // Quick enough for two hammers in the air: thwack, thwack.
            Spell::MagicHammer => 0.25,
            // The rate of fire: about eleven sparks a second.
            Spell::SparkBarrage => 0.09,
        }
    }
}

/// Extra spell damage at `level`.
pub fn level_bonus(level: u32) -> i32 {
    ((level.saturating_sub(1)) / tuning::LEVELS_PER_BONUS) as i32
}
