//! Save and resume. The run is written every few seconds, on every new
//! floor, and when the app goes to the background; the next launch picks it
//! up where it was, auto mode and banked level-ups included. Death deletes
//! it: a new run starts from scratch.
//!
//! Where it lives: a file in the platform's data folder on desktop
//! (`ZRPG_SAVE_DIR` overrides), the page's `localStorage` on the web, and
//! the app's private storage on Android. Scripted runs, runs with a chosen
//! seed and tools never read or write it.

use bevy::prelude::*;
use bevy::window::AppLifecycle;
use zrpg_core::{Event, Game};

use crate::auto::AutoMode;
use crate::{GameEvent, Run};

/// Seconds between routine saves.
const SAVE_EVERY_SECS: f32 = 5.0;
/// Our own header byte, ahead of the core's save (which is versioned too).
const VERSION: u8 = 1;

/// Saving is on (normal play).
#[derive(Resource)]
pub struct Saving;

/// A run read back from storage, and whether auto mode was playing it.
pub struct Resumed {
    pub game: Game,
    pub auto: bool,
}

pub struct PersistPlugin;

impl Plugin for PersistPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, save_now_and_then);
    }
}

/// The saved run, if there is a valid one.
pub fn resume() -> Option<Resumed> {
    let bytes = store::read()?;
    let (&version, rest) = bytes.split_first()?;
    let (&auto, game) = rest.split_first()?;
    if version != VERSION {
        return None;
    }
    let game = Game::load(game)?;
    game.player.is_alive().then_some(Resumed {
        game,
        auto: auto != 0,
    })
}

fn encode(game: &Game, auto: bool) -> Vec<u8> {
    let mut bytes = vec![VERSION, auto as u8];
    bytes.extend(game.save());
    bytes
}

#[allow(clippy::too_many_arguments)]
fn save_now_and_then(
    saving: Option<Res<Saving>>,
    time: Res<Time>,
    run: Res<Run>,
    auto: Res<AutoMode>,
    mut events: MessageReader<GameEvent>,
    mut lifecycle: MessageReader<AppLifecycle>,
    mut exit: MessageReader<AppExit>,
    mut since: Local<f32>,
) {
    if saving.is_none() {
        events.clear();
        return;
    }
    *since += time.delta_secs();
    let mut now = *since >= SAVE_EVERY_SECS;
    for GameEvent(event) in events.read() {
        match event {
            Event::PlayerDied => {
                store::delete();
                *since = 0.0;
                return;
            }
            Event::Descended { .. } | Event::LeveledUp { .. } | Event::PerkTaken { .. } => {
                now = true
            }
            _ => {}
        }
    }
    // Going to the background may be the last chance on a phone.
    if lifecycle
        .read()
        .any(|l| matches!(l, AppLifecycle::WillSuspend | AppLifecycle::Suspended))
        || exit.read().count() > 0
    {
        now = true;
    }
    if now && run.player.is_alive() {
        *since = 0.0;
        store::write(&encode(&run, auto.0));
    }
}

#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
mod store {
    use std::path::PathBuf;

    fn dir() -> Option<PathBuf> {
        if let Some(dir) = std::env::var_os("ZRPG_SAVE_DIR") {
            return Some(dir.into());
        }
        let home = |rest: &str| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(rest));
        if cfg!(target_os = "windows") {
            std::env::var_os("APPDATA").map(|d| PathBuf::from(d).join("zrpg"))
        } else if cfg!(target_os = "macos") {
            home("Library/Application Support/zrpg")
        } else {
            std::env::var_os("XDG_DATA_HOME")
                .map(|d| PathBuf::from(d).join("zrpg"))
                .or_else(|| home(".local/share/zrpg"))
        }
    }

    pub(super) fn path() -> Option<PathBuf> {
        dir().map(|d| d.join("run.sav"))
    }

    pub(super) fn read() -> Option<Vec<u8>> {
        std::fs::read(path()?).ok()
    }

    pub(super) fn write(bytes: &[u8]) {
        let Some(path) = path() else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        // Write aside, then swap in, so a crash never leaves half a save.
        let tmp = path.with_extension("tmp");
        if std::fs::write(&tmp, bytes).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
    }

    pub(super) fn delete() {
        if let Some(path) = path() {
            let _ = std::fs::remove_file(path);
        }
    }
}

#[cfg(target_os = "android")]
mod store {
    use std::path::PathBuf;

    fn path() -> Option<PathBuf> {
        let app = bevy::android::ANDROID_APP.get()?;
        Some(app.internal_data_path()?.join("run.sav"))
    }

    pub(super) fn read() -> Option<Vec<u8>> {
        std::fs::read(path()?).ok()
    }

    pub(super) fn write(bytes: &[u8]) {
        let Some(path) = path() else { return };
        let tmp = path.with_extension("tmp");
        if std::fs::write(&tmp, bytes).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
    }

    pub(super) fn delete() {
        if let Some(path) = path() {
            let _ = std::fs::remove_file(path);
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod store {
    const KEY: &str = "zrpg.run";

    fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok()?
    }

    pub(super) fn read() -> Option<Vec<u8>> {
        let hex = storage()?.get_item(KEY).ok()??;
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok())
            .collect()
    }

    pub(super) fn write(bytes: &[u8]) {
        let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        if let Some(s) = storage() {
            let _ = s.set_item(KEY, &hex);
        }
    }

    pub(super) fn delete() {
        if let Some(s) = storage() {
            let _ = s.remove_item(KEY);
        }
    }
}

#[cfg(all(test, not(any(target_arch = "wasm32", target_os = "android"))))]
mod tests {
    use super::*;

    #[test]
    fn a_run_survives_the_trip_through_storage() {
        let dir = std::env::temp_dir().join(format!("zrpg-save-test-{}", std::process::id()));
        // SAFETY: this test is the only one that touches the variable.
        unsafe { std::env::set_var("ZRPG_SAVE_DIR", &dir) };
        let mut game = Game::new(77);
        game.player.hp -= 3;
        store::write(&encode(&game, true));
        let back = resume().expect("resumes");
        assert!(back.auto);
        assert_eq!(back.game.player, game.player);
        assert_eq!(back.game.run_seed, 77);
        // A dead run is not resumed; a deleted one is gone.
        game.player.hp = 0;
        store::write(&encode(&game, false));
        assert!(resume().is_none());
        store::delete();
        assert!(store::read().is_none());
        let _ = std::fs::remove_dir_all(dir);
    }
}
