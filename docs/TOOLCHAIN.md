# Toolchain

| Component | Version | Why |
|-----------|---------|-----|
| Rust      | 1.97.0 (pinned in `rust-toolchain.toml`) | Stable; satisfies Bevy 0.19's MSRV (1.95). |
| Edition   | 2024 | Current stable edition. |
| Bevy      | 0.19.1 | Latest patch release of the newest Bevy line our toolchain supports. Bevy 0.20 needs Rust 1.97.1 and is weeks old; we move up once its first patch release lands. |

`zrpg_core` has **no dependencies** and is `no_std` + `alloc`. Keep it that way:
it is the part that has to survive ports (Android, WebAssembly, and someday N64).

## Bevy features

Bevy's default features are off. `crates/zrpg/Cargo.toml` lists exactly what
we use: windowing (winit, X11, Wayland), keyboard, gamepads (gilrs), the core
3D pipeline with unlit `StandardMaterial` and distance fog, sprites (to
present the low-res frame), UI/text (for the HUD), and PNG (screenshots).
No glTF, audio, animation, picking, scenes or post-processing yet. Add a
feature only when something needs it.

## Linux build dependencies

Debian/Ubuntu:

```sh
sudo apt-get install -y pkg-config libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev
```

`libasound2-dev` is not needed by our feature set today, but you will want it
the day audio lands. A Vulkan or GL driver is required to run the game
(Mesa is fine; `mesa-vulkan-drivers` provides a software fallback).

## Build profiles

- `dev`: our code at `opt-level = 1`, dependencies at `opt-level = 3`, so debug
  builds run at full speed. First build takes a while; later ones are quick.
- `release`: thin LTO, one codegen unit.
