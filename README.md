# ZRPG

*An RPG for busy people.*

ZRPG is an open-source, minimalist, first-person dungeon crawler roguelike in
the spirit of Dungeon Hack, Lands of Lore, Eye of the Beholder and Anvil of
Dawn: grid movement, 90° turns, real-time combat, and endless seeded floors.
Pick it up, play a floor, put it down.

![Floor 1, seed 7](docs/screenshot.png)

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
| Restart (new seed) | R            | Select / Back        |

Hold a direction to keep walking. Taps made mid-step are buffered.

## Layout

```
crates/
  zrpg_core/   Portable rules: map, generator, RNG, movement. no_std, no deps, no Bevy.
  zrpg/        Bevy front-end: low-res screen, level meshes, camera, input.
docs/          Toolchain and roadmap notes.
```

The front-end sends `Command`s into `zrpg_core::Game` and animates the
`Event`s that come back. Gameplay never touches Bevy; Bevy never decides
gameplay.

## Development

```sh
cargo test                                   # core rules + mesh sanity checks
cargo run -p zrpg_core --example ascii 7     # print generated floors as ASCII
ZRPG_AUTOPILOT="w e shot quit" cargo run     # scripted input + screenshot (zrpg-0.png)
```

## Status: Vertical Slice 0.1

- [x] Connected, seeded 12×12 floors (rooms + corridors)
- [x] First-person walls, floors, ceilings at 320×240
- [x] Grid movement, turning, strafing, collision
- [x] Keyboard and gamepad
- [ ] Billboard enemy that approaches and attacks
- [ ] Player attack, Health and Energy HUD
- [ ] Stairs to the next floor (stairs are generated and drawn, not yet usable)
- [ ] Death and instant restart (restart key exists)

## License

MIT OR Apache-2.0.
