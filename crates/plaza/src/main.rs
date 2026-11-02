//! The interactive skate plaza. In production it runs as WASM inside the shell
//! page that `crates/classic` writes; natively it opens a window for development.

mod assets;
mod bridge;
mod camera;
mod character;
mod feedback;
mod input;
mod loading;
#[cfg(not(target_arch = "wasm32"))]
mod og_image;
mod palette;
mod plaza;
mod skater;
mod spots;

use avian3d::prelude::*;
use bevy::{asset::AssetMetaCheck, prelude::*};

/// Rendering at a phone's full 3× pixel ratio costs far more than it shows.
const MAX_PIXEL_RATIO: f32 = 2.0;

/// Trigger zones sit on their own layer: only the skater sets them off, and
/// the skater's ground probe looks past them.
#[derive(PhysicsLayer, Default)]
enum Layer {
    #[default]
    Default,
    Zone,
}

fn main() {
    let window = Window {
        title: "Plaza".into(),
        canvas: Some("#plaza".into()),
        fit_canvas_to_parent: true,
        // Keep browser shortcuts and Tab focus working.
        prevent_default_event_handling: false,
        ..default()
    };
    #[cfg(not(target_arch = "wasm32"))]
    let og_image = og_image::requested();
    #[cfg(not(target_arch = "wasm32"))]
    let window = match og_image {
        Some(_) => Window {
            resolution: og_image::window_resolution(),
            ..window
        },
        None => window,
    };

    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(window),
                ..default()
            })
            .set(AssetPlugin {
                meta_check: AssetMetaCheck::Never,
                ..default()
            }),
        PhysicsPlugins::default(),
        assets::plugin,
        loading::plugin,
        input::plugin,
        plaza::plugin,
        skater::plugin,
        character::plugin,
        spots::plugin,
        feedback::plugin,
        camera::plugin,
    ))
    .add_systems(Update, cap_pixel_ratio);
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(path) = og_image {
        app.add_plugins(og_image::plugin(path));
    }
    app.run();
}

fn cap_pixel_ratio(mut windows: Query<&mut Window, Changed<Window>>) {
    for mut window in &mut windows {
        let capped = window.resolution.base_scale_factor().min(MAX_PIXEL_RATIO);
        if window.resolution.scale_factor_override() != Some(capped) {
            window.resolution.set_scale_factor_override(Some(capped));
        }
    }
}
