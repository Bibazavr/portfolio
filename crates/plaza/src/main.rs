//! The interactive skate plaza. In production it runs as WASM inside the shell
//! page that `crates/classic` writes; natively it opens a window for development.

mod bridge;
mod camera;
mod input;
mod palette;
mod plaza;
mod skater;
mod spots;

use avian3d::prelude::*;
use bevy::{asset::AssetMetaCheck, prelude::*};

/// Rendering at a phone's full 3× pixel ratio costs far more than it shows.
const MAX_PIXEL_RATIO: f32 = 2.0;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Plaza".into(),
                        canvas: Some("#plaza".into()),
                        fit_canvas_to_parent: true,
                        // Keep browser shortcuts and Tab focus working.
                        prevent_default_event_handling: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                }),
            PhysicsPlugins::default(),
            input::plugin,
            plaza::plugin,
            skater::plugin,
            spots::plugin,
            camera::plugin,
        ))
        .add_systems(Update, cap_pixel_ratio)
        .run();
}

fn cap_pixel_ratio(mut windows: Query<&mut Window, Changed<Window>>) {
    for mut window in &mut windows {
        let capped = window.resolution.base_scale_factor().min(MAX_PIXEL_RATIO);
        if window.resolution.scale_factor_override() != Some(capped) {
            window.resolution.set_scale_factor_override(Some(capped));
        }
    }
}
