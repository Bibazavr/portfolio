//! Loads the models with bevy_asset_loader before the plaza is built, then
//! tells the page to drop its loading screen. The page shows the download
//! progress of the WASM itself, which is most of the wait.

use bevy::prelude::*;
use bevy_asset_loader::prelude::*;

use crate::{assets::Models, bridge};

/// The first frames after the plaza is built still compile shaders; the
/// loading screen stays up for these.
const SETTLE_FRAMES: u32 = 10;

#[derive(States, Default, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Phase {
    #[default]
    Loading,
    Playing,
}

pub fn plugin(app: &mut App) {
    app.init_state::<Phase>()
        .add_loading_state(
            LoadingState::new(Phase::Loading)
                .continue_to_state(Phase::Playing)
                .load_collection::<Models>(),
        )
        .add_systems(Update, announce_ready.run_if(in_state(Phase::Playing)));
}

fn announce_ready(mut frames: Local<u32>) {
    *frames += 1;
    if *frames == SETTLE_FRAMES {
        bridge::ready();
    }
}
