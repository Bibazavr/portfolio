//! The link to the shell page around the canvas: Bevy tells it which spot's
//! panel to show, and its touch stick tells Bevy where to skate. Natively there
//! is no page, so panels are logged and the stick stays centred.

use bevy::math::Vec2;
use content::Spot;

pub fn show(spot: Spot) {
    imp::show(&spot.id());
}

pub fn hide() {
    imp::hide();
}

/// The touch stick, with `y` pointing up, or zero when it isn't held.
pub fn stick() -> Vec2 {
    imp::stick()
}

#[cfg(target_arch = "wasm32")]
mod imp {
    use std::cell::Cell;

    use bevy::math::Vec2;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        // `window.plazaBridge`, which the page defines before the plaza starts.
        #[wasm_bindgen(js_namespace = plazaBridge)]
        pub fn show(spot: &str);
        #[wasm_bindgen(js_namespace = plazaBridge)]
        pub fn hide();
    }

    thread_local! {
        static STICK: Cell<Vec2> = const { Cell::new(Vec2::ZERO) };
    }

    /// Called by the page's touch stick.
    #[wasm_bindgen]
    pub fn set_stick(x: f32, y: f32) {
        STICK.set(Vec2::new(x, y).clamp_length_max(1.0));
    }

    pub fn stick() -> Vec2 {
        STICK.get()
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use bevy::{log::info, math::Vec2};

    pub fn show(spot: &str) {
        info!("panel: {spot}");
    }

    pub fn hide() {
        info!("panel: none");
    }

    pub fn stick() -> Vec2 {
        Vec2::ZERO
    }
}
