//! What the player wants the skater to do, independent of the device it comes from.

use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct SkaterInput {
    /// 1 pushes, -1 brakes.
    pub throttle: f32,
    /// 1 turns left, -1 turns right.
    pub steer: f32,
}

pub fn plugin(app: &mut App) {
    app.init_resource::<SkaterInput>()
        .add_systems(PreUpdate, read_keyboard.after(bevy::input::InputSystems));
}

fn read_keyboard(keys: Res<ButtonInput<KeyCode>>, mut input: ResMut<SkaterInput>) {
    let axis = |positive: [KeyCode; 2], negative: [KeyCode; 2]| {
        f32::from(keys.any_pressed(positive)) - f32::from(keys.any_pressed(negative))
    };
    input.throttle = axis(
        [KeyCode::KeyW, KeyCode::ArrowUp],
        [KeyCode::KeyS, KeyCode::ArrowDown],
    );
    input.steer = axis(
        [KeyCode::KeyA, KeyCode::ArrowLeft],
        [KeyCode::KeyD, KeyCode::ArrowRight],
    );
}
