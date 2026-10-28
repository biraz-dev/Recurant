//! Game state and the loop the front-end drives.
//!
//! The front-end calls [`Game::apply`] for each player command and
//! [`Game::tick`] once per frame. Both resolve instantly and append
//! [`Event`]s, which the front-end animates at whatever pace it likes.

use alloc::vec::Vec;

use crate::dungeon::{Floor, GenParams, Place};
use crate::enemy::{Enemy, EnemyId, EnemyKind, EnemyState, tuning as enemy_tuning};
use crate::grid::{Facing, GridPos, Step, Turn};
use crate::map::Tile;
use crate::rng::{Rng, mix64};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Step(Step),
    Turn(Turn),
    Attack,
    /// Take the stairs down. Only works while standing on them.
    Descend,
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
        }
    }

    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }

    /// Ready to swing and able to pay for it.
    pub fn can_attack(&self) -> bool {
        self.attack_cooldown <= 0.0 && self.energy >= tuning::ATTACK_COST
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
    rng: Rng,
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
        let p = &mut self.player;
        p.pos = self.floor.start;
        p.facing = self.floor.start_facing;
        p.hp = (p.hp + tuning::DESCEND_HEAL).min(p.max_hp);
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
        }
    }

    fn attack(&mut self, out: &mut Vec<Event>) {
        let p = &mut self.player;
        if p.attack_cooldown > 0.0 {
            return;
        }
        if p.energy < tuning::ATTACK_COST {
            out.push(Event::Exhausted);
            return;
        }
        p.energy -= tuning::ATTACK_COST;
        p.attack_cooldown = tuning::ATTACK_COOLDOWN;
        let target = p.pos + p.facing;
        // A parapet between you and it is out of reach.
        let reachable = self.floor.map.within_reach(p.pos, target);

        let Some(i) = self
            .enemies
            .iter()
            .position(|e| e.pos == target && reachable)
        else {
            out.push(Event::Swung { hit: None });
            return;
        };
        let damage = self.rng.range(tuning::DAMAGE_MIN, tuning::DAMAGE_MAX);
        let enemy = &mut self.enemies[i];
        out.push(Event::Swung {
            hit: Some(enemy.id),
        });
        enemy.hp -= damage;
        // A hit wakes sleepers and interrupts a wind-up.
        let staggered = enemy.is_winding_up();
        if staggered {
            enemy.state = EnemyState::Recover {
                left: enemy_tuning::STAGGER,
            };
        } else if enemy.state == EnemyState::Dormant {
            enemy.state = EnemyState::Chasing;
        }
        out.push(Event::EnemyHurt {
            id: enemy.id,
            damage,
            staggered,
        });
        if enemy.hp <= 0 {
            out.push(Event::EnemyDied {
                id: enemy.id,
                at: enemy.pos,
            });
            self.enemies.swap_remove(i);
        }
    }

    /// Advance real time by `dt` seconds.
    pub fn tick(&mut self, dt: f32, out: &mut Vec<Event>) {
        if !self.player.is_alive() {
            return;
        }
        let p = &mut self.player;
        p.attack_cooldown = (p.attack_cooldown - dt).max(0.0);
        p.energy = (p.energy + tuning::ENERGY_REGEN * dt).min(p.max_energy);

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
                // Stepping away during the tell dodges the blow.
                let damage = if adjacent {
                    self.rng.range(stats.damage_min, stats.damage_max)
                } else {
                    0
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
            Enemy::of_kind(i as EnemyId, p, kind_for(floor.place, depth, roll))
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
