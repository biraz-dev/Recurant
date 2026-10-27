//! Android entry point. `#[bevy_main]` exports the `android_main` symbol that
//! GameActivity calls once the native library is loaded.

use bevy::prelude::*;

#[bevy_main]
fn main() {
    zrpg::app().run();
}
