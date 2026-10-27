# Platforms

All three builds run the same `zrpg` library; platform differences live in
`crates/zrpg/src/platform.rs` (window, seed, auto-mode flag) and in the
Android key codes in `crates/zrpg/src/input.rs`.

## Linux (desktop)

```sh
cargo run --release -- [--seed N] [--auto]
```

`ZRPG_SEED` and `ZRPG_AUTO=1` work too. Packages: see [TOOLCHAIN.md](TOOLCHAIN.md).

## Web (WebAssembly + WebGL2)

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129   # must match Cargo.lock
scripts/build-web.sh
python3 -m http.server --directory target/web 8000  # then open http://localhost:8000
```

- The page is `web/index.html`; the game draws into its `<canvas id="zrpg">`,
  which fills the window. Click it if the keyboard doesn't respond.
- URL options: `?seed=42`, `?auto`, or both (`?seed=42&auto`).
- Gamepads work through the browser's Gamepad API (press a button first;
  browsers hide pads until then).
- Size: about 26 MB, about 6.5 MB gzipped (most of it is Bevy's renderer and
  text stack). Serve it compressed. Don't run Ubuntu's old `wasm-opt`
  (binaryen 108) on it; it breaks wasm-bindgen's reference table.
- Built with the `web-release` profile (`opt-level = "s"`, fat LTO).

## Android (handhelds first)

```sh
rustup target add aarch64-linux-android
cargo install cargo-ndk
export ANDROID_HOME=...  ANDROID_NDK_HOME=...       # SDK with platform 35, any recent NDK
scripts/build-android.sh                             # -> android/app/build/outputs/apk/debug/app-debug.apk
adb install -r android/app/build/outputs/apk/debug/app-debug.apk
```

- `crates/zrpg_android` is a `cdylib`; `#[bevy_main]` provides the
  `android_main` entry point that GameActivity calls.
- `android/` is a plain Gradle project (AGP 8.7.3, Gradle 8.11.1, JDK 17,
  minSdk 29, targetSdk 35). It depends on `androidx.games:games-activity:4.4.0`,
  the version the `android-activity` crate (0.6.x) is built against; keep the
  two in step when Bevy upgrades. Don't enable prefab.
- The Gradle wrapper scripts are not committed; Android Studio uses the
  pinned version from `gradle/wrapper/gradle-wrapper.properties`, or run
  `gradle wrapper` once to generate `gradlew`.
- The APK is debug-signed for sideloading. The game library inside is built
  in release mode, so it runs at full speed.
- Landscape only, fullscreen, system bars hidden. Back (which many handhelds
  send for B) never closes the game.
- Controls: built-in and Bluetooth pads arrive as Android key events. The
  D-pad, A/X/R2 (attack), L1/R1 (strafe), Y (auto) and Select (restart) are
  bound. **Analog sticks are not read on Android yet** (winit doesn't expose
  joystick axes); most handhelds can map the left stick to the D-pad.
- No touch controls yet.
- CI (`.github/workflows/ci.yml`) builds the APK on every push and uploads
  it as the `zrpg-android` artifact.
