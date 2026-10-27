# ZRPG

*An RPG for busy people.*

ZRPG is an open-source, minimalist, first-person dungeon crawler roguelike in
the spirit of Dungeon Hack, Lands of Lore, Eye of the Beholder and Anvil of
Dawn: grid movement, 90° turns, real-time combat, and endless seeded floors.
Pick it up, play a floor, put it down.

![A zombie shambles down the corridor (seed 7)](docs/screenshot.png)

Built with Rust and [Bevy](https://bevy.org). Renders at 320×240, scaled up
with crisp pixels. Runs on Linux, in the browser, and on Android handhelds.

## Run

```sh
cargo run --release                 # random seed
cargo run --release -- --seed 42    # a specific dungeon
cargo run --release -- --auto       # let it play itself
scripts/build-web.sh                # browser build in target/web/
scripts/build-android.sh            # APK (needs the Android SDK + NDK)
```

On the web, `?seed=42` and `?auto` in the page URL do the same as the flags.
See [docs/PLATFORMS.md](docs/PLATFORMS.md) for web and Android details and
[docs/TOOLCHAIN.md](docs/TOOLCHAIN.md) for versions and Linux packages.

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
| Auto mode on/off | Tab            | Y                    |
| Restart (new seed) | R (or Attack once dead) | Select / Back |

Hold a direction to keep walking; hold Attack to keep swinging. Taps made
mid-step are buffered.

## Combat

Real time, on cooldowns. Each swing costs Energy (yellow bar), which refills
on its own; swing too eagerly and you'll be left flailing. Enemies sleep until
you come close, then chase. When one is next to you it **winds up** (rears up
and glows orange) before it strikes:

- **Step away** during the wind-up and the blow misses.
- **Hit it** during the wind-up and it staggers, cancelling the strike.

Health (red bar) does not regenerate. Reach zero and you die; press Attack to
rise again in a fresh dungeon.

## Going deeper

Every floor has one staircase, as far from the start as the floor allows,
marked by an archway over steps leading down. Walk onto it to descend; there
is no button to press. Each new floor heals 5 Health. Floors never run out,
and a seed always produces the same sequence of floors.

## Auto mode

Press Tab (gamepad: Y) and the game plays for you: it hunts awake enemies
while healthy, staggers wind-ups and backs out of blows it can't stop, and
heads for the stairs when hurt. It picks itself up after dying. Press any move
or attack to take over again; press Tab to hand back. The bot lives in
`zrpg_core::auto` and only issues the same commands a player can.

## Layout

```
crates/
  zrpg_core/     Portable rules: map, generator, RNG, movement, combat, enemy
                 AI, auto-mode bot. no_std, no deps, no Bevy.
  zrpg/          Bevy front-end (library + desktop/web binary): low-res screen,
                 level meshes, camera, billboards, HUD, input.
  zrpg_android/  Android entry point (GameActivity).
android/         Gradle project that packages the APK.
web/             Page that hosts the browser build.
art/             Source art (references).  tools/  Asset scripts.
docs/            Toolchain and platform notes.
```

Each frame the front-end sends player `Command`s into `zrpg_core::Game`,
calls `Game::tick(dt)`, and animates the `Event`s that come back. All tuning
numbers live in `tuning` modules in `zrpg_core` (`game.rs`, `enemy.rs`).
Gameplay never touches Bevy; Bevy never decides gameplay.

## Development

```sh
cargo test                                   # rules, bot, mesh sanity checks
cargo run -p zrpg_core --example ascii 7     # print generated floors as ASCII
ZRPG_SCRIPT="w e shot quit" cargo run        # scripted input + screenshot (zrpg-0.png)
python3 tools/make_sprite.py art/zombie-reference.png crates/zrpg/assets/zombie.png --height 64
```

`make_sprite.py` turns a large reference on a black background into a
game sprite: background removed, narrow gaps closed, downscaled to a small
palette with binary alpha and a dark outline. Sprites are embedded in the
binary, so no platform loads files at runtime.

## Status

Vertical Slice 0.1 is complete; since then: web and Android builds, auto mode,
and the zombie.

### Vertical Slice 0.1

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
