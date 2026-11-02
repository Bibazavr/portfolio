//! The link to the shell page around the canvas. Bevy tells it which spot's
//! panel to show, which sounds to play and which tricks scored; the page's
//! touch controls tell Bevy where to skate and when to ollie. Natively there
//! is no page, so these are logged or do nothing.

use bevy::math::Vec2;
use content::Spot;

pub fn show(spot: Spot) {
    imp::show(&spot.id());
}

pub fn hide() {
    imp::hide();
}

/// A one-shot sound: `ollie` or `land`.
pub fn sound(name: &str) {
    imp::sound(name);
}

/// Drives the rolling and grinding sounds; `speed` is 0 when still, 1 at top speed.
pub fn motion(speed: f32, rolling: bool, grinding: bool) {
    imp::motion(speed, rolling, grinding);
}

pub fn trick(kind: &str, points: u32, total: u32) {
    imp::trick(kind, points, total);
}

/// The touch stick, with `y` pointing up, or zero when it isn't held.
pub fn stick() -> Vec2 {
    imp::stick()
}

/// Whether the touch ollie button was pressed since the last call.
pub fn take_ollie() -> bool {
    imp::take_ollie()
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
        #[wasm_bindgen(js_namespace = plazaBridge)]
        pub fn sound(name: &str);
        #[wasm_bindgen(js_namespace = plazaBridge)]
        pub fn motion(speed: f32, rolling: bool, grinding: bool);
        #[wasm_bindgen(js_namespace = plazaBridge)]
        pub fn trick(kind: &str, points: u32, total: u32);
    }

    thread_local! {
        static STICK: Cell<Vec2> = const { Cell::new(Vec2::ZERO) };
        static OLLIE: Cell<bool> = const { Cell::new(false) };
    }

    /// Called by the page's touch stick.
    #[wasm_bindgen]
    pub fn set_stick(x: f32, y: f32) {
        STICK.set(Vec2::new(x, y).clamp_length_max(1.0));
    }

    /// Called by the page's ollie button.
    #[wasm_bindgen]
    pub fn ollie() {
        OLLIE.set(true);
    }

    pub fn stick() -> Vec2 {
        STICK.get()
    }

    pub fn take_ollie() -> bool {
        OLLIE.replace(false)
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

    pub fn sound(_name: &str) {}

    pub fn motion(_speed: f32, _rolling: bool, _grinding: bool) {}

    pub fn trick(kind: &str, points: u32, total: u32) {
        info!("{kind} +{points} = {total}");
    }

    pub fn stick() -> Vec2 {
        Vec2::ZERO
    }

    pub fn take_ollie() -> bool {
        false
    }
}
