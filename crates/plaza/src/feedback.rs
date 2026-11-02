//! What the page shows and plays about the skater: the running score, and the
//! rolling and grinding sounds that follow the board.

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::{
    bridge,
    skater::{Board, Motion, Skater, Trick},
};

/// Speed at which the rolling sound is at full volume.
const LOUDEST_SPEED: f32 = 9.0;

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (tally, report_motion));
}

fn tally(mut tricks: MessageReader<Trick>, mut total: Local<u32>) {
    for trick in tricks.read() {
        *total += trick.points;
        bridge::trick(trick.kind.id(), trick.points, *total);
    }
}

fn report_motion(skater: Single<(&Board, &LinearVelocity), With<Skater>>) {
    let (board, velocity) = *skater;
    let speed = (velocity.length() / LOUDEST_SPEED).min(1.0);
    let (rolling, grinding) = match board.motion {
        Motion::Rolling => (true, false),
        Motion::Air(_) => (false, false),
        Motion::Grinding(_) => (false, true),
    };
    bridge::motion(speed, rolling, grinding);
}
