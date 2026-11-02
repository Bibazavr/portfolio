//! What the player wants the skater to do, independent of the device it comes from.

use bevy::prelude::*;

use crate::bridge;

/// Stick travel below this counts as centred.
const DEAD_ZONE: f32 = 0.15;
/// An ollie pressed just before the board can pop still counts, for this long.
const OLLIE_BUFFER: f32 = 0.15;

#[derive(Resource, Default)]
pub struct SkaterInput {
    /// 1 pushes, -1 brakes.
    pub throttle: f32,
    /// 1 turns left, -1 turns right.
    pub steer: f32,
    /// When an ollie was asked for and not yet done; `take` it to do it.
    pub ollie: Option<f32>,
}

pub fn plugin(app: &mut App) {
    app.init_resource::<SkaterInput>()
        .add_systems(PreUpdate, read_input.after(bevy::input::InputSystems));
}

fn read_input(
    time: Res<Time<Real>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut input: ResMut<SkaterInput>,
) {
    let axis = |positive: [KeyCode; 2], negative: [KeyCode; 2]| {
        f32::from(keys.any_pressed(positive)) - f32::from(keys.any_pressed(negative))
    };
    let mut stick = bridge::stick();
    if stick.length() < DEAD_ZONE {
        stick = Vec2::ZERO;
    }
    let throttle = axis(
        [KeyCode::KeyW, KeyCode::ArrowUp],
        [KeyCode::KeyS, KeyCode::ArrowDown],
    );
    let steer = axis(
        [KeyCode::KeyA, KeyCode::ArrowLeft],
        [KeyCode::KeyD, KeyCode::ArrowRight],
    );
    input.throttle = (throttle + stick.y).clamp(-1.0, 1.0);
    input.steer = (steer - stick.x).clamp(-1.0, 1.0);

    let now = time.elapsed_secs();
    // `take_ollie` runs first so a button press is never left behind for later.
    if bridge::take_ollie() | keys.just_pressed(KeyCode::Space) {
        input.ollie = Some(now);
    } else if input
        .ollie
        .is_some_and(|pressed| now - pressed > OLLIE_BUFFER)
    {
        input.ollie = None;
    }
}
