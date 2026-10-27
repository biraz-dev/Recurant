# ZRPG

*An RPG for busy people.*

ZRPG is an open-source, minimalist, first-person dungeon crawler roguelike in
the spirit of Dungeon Hack, Lands of Lore, Eye of the Beholder and Anvil of
Dawn: grid movement, 90° turns, real-time combat, and endless seeded floors.
Pick it up, play a floor, put it down.

![A slime winds up to strike (seed 7)](docs/screenshot.png)

Built with Rust and [Bevy](https://bevy.org). Renders at 320×240, scaled up
with crisp pixels. Linux first; Android handhelds and WebAssembly next.

## Run

```sh
cargo run --release              # random seed
cargo run --release -- --seed 42 # a specific dungeon
```

See [docs/TOOLCHAIN.md](docs/TOOLCHAIN.md) for versions and Linux packages.

## Controls

| Action        | Keyboard          | Gamepad              |
|---------------|-------------------|----------------------|
| Forward       | W / ↑             | D-pad ↑, left stick ↑ |
| Back          | S / ↓             | D-pad ↓, left stick ↓ |
| Turn left     | Q / ←             | D-pad ←, left stick ← |
| Turn right    | E / →             | D-pad →, left stick → |
| Strafe left   | A                 | LB, right stick ←    |
| Strafe right  | D                 | RB, right stick →    |
| Attack        | Space / F         | A, X, RT             |
| Restart (new seed) | R (or Attack once dead) | Select / Back |

Hold a direction to keep walking; hold Attack to keep swinging. Taps made
mid-step are buffered.

## Combat

Real time, on cooldowns. Each swing costs Energy (yellow bar), which refills
on its own; swing too eagerly and you'll be left flailing. Enemies sleep until
you come close, then chase. When one is next to you it **winds up** (swells and
glows orange) before it strikes:

- **Step away** during the wind-up and the blow misses.
- **Hit it** during the wind-up and it staggers, cancelling the strike.

Health (red bar) does not regenerate. Reach zero and you die; press Attack to
rise again in a fresh dungeon.

## Going deeper

Every floor has one staircase, as far from the start as the floor allows,
marked by an archway over steps leading down. Walk onto it to descend; there
is no button to press. Each new floor heals 5 Health. Floors never run out,
and a seed always produces the same sequence of floors.

## Layout

```
crates/
  zrpg_core/   Portable rules: map, generator, RNG, movement, combat, enemy AI.
               no_std, no deps, no Bevy.
  zrpg/        Bevy front-end: low-res screen, level meshes, camera, billboards,
               HUD, input.
docs/          Toolchain and roadmap notes.
```

Each frame the front-end sends player `Command`s into `zrpg_core::Game`,
calls `Game::tick(dt)`, and animates the `Event`s that come back. All tuning
numbers live in `tuning` modules in `zrpg_core` (`game.rs`, `enemy.rs`).
Gameplay never touches Bevy; Bevy never decides gameplay.

## Development

```sh
cargo test                                   # core rules + mesh sanity checks
cargo run -p zrpg_core --example ascii 7     # print generated floors as ASCII
ZRPG_AUTOPILOT="w e shot quit" cargo run     # scripted input + screenshot (zrpg-0.png)
```

## Status: Vertical Slice 0.1 (complete)

- [x] Connected, seeded 12×12 floors (rooms + corridors)
- [x] First-person walls, floors, ceilings at 320×240
- [x] Grid movement, turning, strafing, collision
- [x] Keyboard and gamepad
- [x] Billboard enemy that approaches and attacks
- [x] Player attack, Health and Energy HUD
- [x] Stairs to the next floor
- [x] Death and instant restart

## License

MIT OR Apache-2.0.
