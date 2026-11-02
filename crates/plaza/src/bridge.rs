//! The link to the shell page around the canvas. Bevy tells it which spot the
//! skater is at (for screen readers), which link pad offers what, which sounds
//! to play and which tricks scored; the page tells Bevy its language, and its
//! touch controls where to skate and when to ollie. Natively there is no page,
//! so these are logged or do nothing.

use bevy::math::Vec2;
use content::{Lang, Spot};

pub fn show(spot: Spot) {
    imp::show(&spot.id());
}

pub fn hide() {
    imp::hide();
}

/// Points the shown spot's details card at `at`, in CSS pixels from the
/// canvas's top left, or takes it away.
pub fn card(at: Option<Vec2>) {
    match at {
        Some(at) => imp::place(at.x, at.y),
        None => imp::unplace(),
    }
}

/// Offers to open `url`: the page shows a button saying `label`, which Enter presses.
pub fn link(url: &str, label: &str) {
    imp::link(url, label);
}

pub fn unlink() {
    imp::unlink();
}

/// The language the page opened in.
pub fn lang() -> Lang {
    Lang::from_code(&imp::lang()).unwrap_or(Lang::En)
}

/// The language the visitor switched to since the last call, if they did.
pub fn take_lang() -> Option<Lang> {
    imp::take_lang()
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

/// The plaza is built and drawing; the page can drop its loading screen.
pub fn ready() {
    imp::ready();
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
    use content::Lang;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        // `window.plazaBridge`, which the page defines before the plaza starts.
        #[wasm_bindgen(js_namespace = plazaBridge)]
        pub fn show(spot: &str);
        #[wasm_bindgen(js_namespace = plazaBridge)]
        pub fn hide();
        #[wasm_bindgen(js_namespace = plazaBridge)]
        pub fn place(x: f32, y: f32);
        #[wasm_bindgen(js_namespace = plazaBridge)]
        pub fn unplace();
        #[wasm_bindgen(js_namespace = plazaBridge)]
        pub fn link(url: &str, label: &str);
        #[wasm_bindgen(js_namespace = plazaBridge)]
        pub fn unlink();
        #[wasm_bindgen(js_namespace = plazaBridge)]
        pub fn lang() -> String;
        #[wasm_bindgen(js_namespace = plazaBridge)]
        pub fn sound(name: &str);
        #[wasm_bindgen(js_namespace = plazaBridge)]
        pub fn motion(speed: f32, rolling: bool, grinding: bool);
        #[wasm_bindgen(js_namespace = plazaBridge)]
        pub fn trick(kind: &str, points: u32, total: u32);
        #[wasm_bindgen(js_namespace = plazaBridge)]
        pub fn ready();
    }

    thread_local! {
        static STICK: Cell<Vec2> = const { Cell::new(Vec2::ZERO) };
        static OLLIE: Cell<bool> = const { Cell::new(false) };
        static LANG: Cell<Option<Lang>> = const { Cell::new(None) };
    }

    /// Called by the page's language toggle.
    #[wasm_bindgen]
    pub fn set_lang(code: &str) {
        LANG.set(Lang::from_code(code));
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

    pub fn take_lang() -> Option<Lang> {
        LANG.take()
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use bevy::{log::info, math::Vec2};
    use content::Lang;

    pub fn show(spot: &str) {
        info!("panel: {spot}");
    }

    pub fn hide() {
        info!("panel: none");
    }

    pub fn place(_x: f32, _y: f32) {}

    pub fn unplace() {}

    pub fn link(url: &str, label: &str) {
        info!("link: {label} ({url})");
    }

    pub fn unlink() {
        info!("link: none");
    }

    /// `--lang ru` picks the language.
    pub fn lang() -> String {
        let mut args = std::env::args().skip_while(|arg| arg != "--lang");
        args.nth(1).unwrap_or_default()
    }

    pub fn take_lang() -> Option<Lang> {
        None
    }

    pub fn sound(_name: &str) {}

    pub fn motion(_speed: f32, _rolling: bool, _grinding: bool) {}

    pub fn trick(kind: &str, points: u32, total: u32) {
        info!("{kind} +{points} = {total}");
    }

    pub fn ready() {
        info!("ready");
    }

    pub fn stick() -> Vec2 {
        Vec2::ZERO
    }

    pub fn take_ollie() -> bool {
        false
    }
}
