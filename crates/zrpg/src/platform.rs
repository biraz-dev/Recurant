//! The few things that differ between desktop, web and Android.

use bevy::prelude::*;

/// Window settings for this platform (title and size are set by the caller).
pub fn window() -> Window {
    #[cfg(target_arch = "wasm32")]
    {
        Window {
            // Draw into the page's <canvas id="zrpg"> and follow its size.
            canvas: Some("#zrpg".into()),
            fit_canvas_to_parent: true,
            ..default()
        }
    }
    #[cfg(target_os = "android")]
    {
        Window {
            mode: bevy::window::WindowMode::BorderlessFullscreen(
                bevy::window::MonitorSelection::Primary,
            ),
            ..default()
        }
    }
    #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
    {
        Window::default()
    }
}

/// A seed chosen by the player: `--seed N` or `ZRPG_SEED` on desktop,
/// `?seed=N` in the page URL on the web.
pub fn seed_override() -> Option<u64> {
    #[cfg(target_arch = "wasm32")]
    {
        let search = web_sys::window()?.location().search().ok()?;
        search
            .trim_start_matches('?')
            .split('&')
            .find_map(|kv| kv.strip_prefix("seed="))
            .and_then(|s| s.parse().ok())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut args = std::env::args().skip_while(|a| a != "--seed").skip(1);
        args.next()
            .or_else(|| std::env::var("ZRPG_SEED").ok())
            .and_then(|s| s.parse().ok())
    }
}

/// Start in auto mode: `--auto` or `ZRPG_AUTO=1` on desktop, `?auto` in the
/// page URL on the web.
pub fn auto_requested() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.location().search().ok())
            .is_some_and(|q| {
                q.trim_start_matches('?')
                    .split('&')
                    .any(|kv| kv == "auto" || kv.starts_with("auto="))
            })
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::args().any(|a| a == "--auto")
            || std::env::var("ZRPG_AUTO").is_ok_and(|v| v != "0")
    }
}

/// A different number every run. (`std::time::SystemTime` panics in the
/// browser, so the web asks JavaScript for the time instead.)
pub fn clock_seed() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        let ms = js_sys::Date::now();
        let jitter = js_sys::Math::random();
        zrpg_core::rng::mix64(ms.to_bits() ^ jitter.to_bits())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0)
    }
}
