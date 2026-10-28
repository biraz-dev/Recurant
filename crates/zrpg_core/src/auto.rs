//! Auto mode: a small bot that plays through the same [`Command`]s a player
//! issues. It keeps no memory (every call looks at the game afresh), so it can
//! be switched on or off at any moment, and the front-end can ask as often as
//! it likes.
//!
//! The bot's habits, in priority order:
//! 1. On the stairs: go down.
//! 2. Enemy next to it: face it and swing. When the enemy winds up, hit it to
//!    stagger it, or step out of reach if a swing isn't ready.
//! 3. Healthy and an enemy is awake: go and fight it.
//! 4. Otherwise: walk the shortest way to the stairs, facing where it walks.

use crate::enemy::Enemy;
use crate::game::{Command, Game};
use crate::grid::{Facing, GridPos, Step, Turn};

/// Below this share of max Health the bot stops seeking fights and heads
/// for the stairs, which heal.
const CAUTIOUS_HEALTH: f32 = 0.4;

/// The next command, or `None` to wait (for a cooldown, Energy, or a safer
/// moment).
pub fn next_command(game: &Game) -> Option<Command> {
    let p = &game.player;
    if !p.is_alive() {
        return None;
    }
    if game.on_stairs() {
        return Some(Command::Descend);
    }
    let map = &game.floor.map;
    if let Some(enemy) = game.enemies.iter().find(|e| map.within_reach(e.pos, p.pos)) {
        return fight(game, enemy);
    }
    let healthy = p.hp as f32 >= p.max_hp as f32 * CAUTIOUS_HEALTH;
    let quarry = game
        .enemies
        .iter()
        .filter(|e| e.state != crate::enemy::EnemyState::Dormant)
        .min_by_key(|e| e.pos.manhattan(p.pos));
    let target = match quarry {
        Some(e) if healthy => e.pos,
        _ => game.floor.stairs,
    };
    walk_toward(game, target)
}

fn fight(game: &Game, enemy: &Enemy) -> Option<Command> {
    let p = &game.player;
    let dir = direction_between(p.pos, enemy.pos)?;
    let facing = p.facing == dir;
    if enemy.is_winding_up() && !p.can_attack() {
        // No swing ready to interrupt the blow: get out of reach.
        if let Some(step) = dodge(game, enemy) {
            return Some(Command::Step(step));
        }
    }
    if !facing {
        return Some(Command::Turn(turn_toward(p.facing, dir)));
    }
    p.can_attack().then_some(Command::Attack)
}

/// A step that leaves `enemy`'s reach, preferring to back away.
fn dodge(game: &Game, enemy: &Enemy) -> Option<Step> {
    let p = &game.player;
    [
        Step::Back,
        Step::StrafeLeft,
        Step::StrafeRight,
        Step::Forward,
    ]
    .into_iter()
    .find(|step| {
        let dir = step.world_dir(p.facing);
        let to = p.pos + dir;
        game.floor.map.can_step(p.pos, dir)
            && game.enemy_at(to).is_none()
            && to.manhattan(enemy.pos) >= 2
    })
}

fn walk_toward(game: &Game, target: GridPos) -> Option<Command> {
    let p = &game.player;
    let map = &game.floor.map;
    let dist = map.distances(target);
    let here = dist[map.index(p.pos)]?;
    // Any neighbour one step closer will do; trying straight ahead first
    // keeps the bot from turning more than it needs to.
    let dir = [p.facing, p.facing.left(), p.facing.right(), p.facing.back()]
        .into_iter()
        .find(|&d| {
            map.can_step(p.pos, d) && dist[map.index(p.pos + d)].is_some_and(|n| n < here)
        })?;
    if dir != p.facing {
        return Some(Command::Turn(turn_toward(p.facing, dir)));
    }
    let next = p.pos + dir;
    // Never walk back into reach of a blow that is about to land.
    let threatened = game
        .enemies
        .iter()
        .any(|e| e.is_winding_up() && e.pos.manhattan(next) <= 1);
    (!threatened).then_some(Command::Step(Step::Forward))
}

fn direction_between(from: GridPos, to: GridPos) -> Option<Facing> {
    Facing::ALL.into_iter().find(|&d| from + d == to)
}

/// The quicker way round; a half turn goes left.
fn turn_toward(facing: Facing, dir: Facing) -> Turn {
    if facing.right() == dir {
        Turn::Right
    } else {
        Turn::Left
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use super::*;
    use crate::game::Event;

    const DT: f32 = 1.0 / 60.0;
    /// Roughly the front-end's animation times: moves take time, swings don't.
    const STEP_SECS: f32 = 0.16;
    const TURN_SECS: f32 = 0.13;
    const DESCEND_SECS: f32 = 0.85;

    #[derive(Debug, Default)]
    struct Outcome {
        depth: u32,
        died: bool,
        kills: u32,
        /// Longest stretch (seconds) without moving, turning or fighting.
        longest_idle: f32,
    }

    /// Let the bot play `seconds` of game time, pacing commands like the
    /// front-end does.
    fn play(seed: u64, seconds: f32) -> Outcome {
        let mut g = Game::new(seed);
        let mut out = Vec::new();
        let (mut busy, mut idle, mut t) = (0.0f32, 0.0f32, 0.0f32);
        let mut outcome = Outcome::default();
        while t < seconds {
            match next_command(&g) {
                Some(Command::Attack) => g.apply(Command::Attack, &mut out),
                Some(cmd) if busy <= 0.0 => {
                    g.apply(cmd, &mut out);
                    busy = match cmd {
                        Command::Turn(_) => TURN_SECS,
                        Command::Descend => DESCEND_SECS,
                        _ => STEP_SECS,
                    };
                }
                _ => {}
            }
            g.tick(DT, &mut out);
            let active = out
                .iter()
                .any(|e| !matches!(e, Event::EnemyNoticed { .. } | Event::EnemyMoved { .. }));
            idle = if active { 0.0 } else { idle + DT };
            outcome.longest_idle = outcome.longest_idle.max(idle);
            outcome.kills += out
                .iter()
                .filter(|e| matches!(e, Event::EnemyDied { .. }))
                .count() as u32;
            out.clear();
            busy -= DT;
            t += DT;
            if !g.player.is_alive() {
                outcome.died = true;
                break;
            }
        }
        outcome.depth = g.depth;
        outcome
    }

    #[test]
    fn bot_makes_steady_progress() {
        let runs: Vec<Outcome> = (0..60).map(|seed| play(seed, 90.0)).collect();
        for (seed, o) in runs.iter().enumerate() {
            // Never stands around: something happens at least every few seconds.
            assert!(
                o.longest_idle < 4.0,
                "seed {seed}: idle for {:.1}s",
                o.longest_idle
            );
            assert!(
                o.depth >= 2 || o.died,
                "seed {seed}: never left floor 1: {o:?}"
            );
        }
        let deaths = runs.iter().filter(|o| o.died).count();
        let depth: u32 = runs.iter().map(|o| o.depth).sum();
        let kills: u32 = runs.iter().map(|o| o.kills).sum();
        std::println!(
            "bot: {deaths} deaths, {depth} floors, {kills} kills over {} runs",
            runs.len()
        );
        // A competent player, not a perfect one.
        assert!(
            deaths * 4 <= runs.len(),
            "too many deaths: {deaths}/{}",
            runs.len()
        );
        assert!(
            depth as usize >= runs.len() * 5,
            "too shallow: {depth} total floors"
        );
        assert!(kills > 0);
    }

    #[test]
    fn bot_staggers_a_wind_up_or_dodges_it() {
        use crate::enemy::{Enemy, EnemyState};
        use crate::map::{Map, Tile};

        // Corridor with room to back away: player at x=2 facing east, enemy at x=3.
        let mut g = Game::new(0);
        let mut map = Map::filled(8, 3, Tile::Wall);
        for x in 1..7 {
            map.set(GridPos::new(x, 1), Tile::Floor);
        }
        g.floor.map = map;
        g.player.pos = GridPos::new(2, 1);
        g.player.facing = Facing::East;
        let mut enemy = Enemy::new(0, GridPos::new(3, 1));
        enemy.state = EnemyState::WindUp { left: 0.5 };
        g.enemies = alloc::vec![enemy];

        assert_eq!(
            next_command(&g),
            Some(Command::Attack),
            "swing ready: stagger"
        );
        g.player.energy = 0.0;
        assert_eq!(
            next_command(&g),
            Some(Command::Step(Step::Back)),
            "exhausted: back off"
        );
    }

    #[test]
    fn bot_turns_to_face_its_way() {
        let mut g = Game::new(3);
        g.enemies.clear();
        let mut out = Vec::new();
        // From the start the bot only ever turns or walks forward until the stairs.
        for _ in 0..200 {
            match next_command(&g) {
                Some(Command::Descend) => break,
                Some(cmd @ (Command::Turn(_) | Command::Step(Step::Forward))) => {
                    g.apply(cmd, &mut out)
                }
                other => panic!("unexpected {other:?}"),
            }
        }
        assert!(g.on_stairs());
    }
}
