//! The skater: a box on a board that pushes, coasts, brakes and carves.
//! Arcade handling, tuned to be fun rather than realistic.

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::{input::SkaterInput, palette};

#[derive(Component)]
pub struct Skater;

// Metres and seconds.
const TOP_SPEED: f32 = 9.0;
const PUSH: f32 = 7.0;
const BRAKE: f32 = 14.0;
const ROLLING_DRAG: f32 = 0.6;
const TURN_RATE: f32 = 2.6;

const HEIGHT: f32 = 1.2;

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn)
        .add_systems(FixedUpdate, drive);
}

fn spawn(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut part = |size: Vec3, y: f32, color: Color| {
        (
            Mesh3d(meshes.add(Cuboid::from_size(size))),
            MeshMaterial3d(materials.add(color)),
            Transform::from_xyz(0.0, y, 0.0),
        )
    };
    let bottom = -HEIGHT / 2.0;

    commands.spawn((
        Skater,
        Transform::from_xyz(0.0, HEIGHT / 2.0, 12.0),
        Visibility::default(),
        RigidBody::Dynamic,
        Collider::cuboid(0.5, HEIGHT, 0.9),
        // Steering sets the yaw rate; nothing should tip the skater over.
        LockedAxes::new().lock_rotation_x().lock_rotation_z(),
        // `drive` owns the velocity, so contacts shouldn't add drag or bounce.
        Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
        Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
        TransformInterpolation,
        children![
            part(Vec3::new(0.26, 0.05, 0.85), bottom + 0.1, palette::ACCENT),
            part(Vec3::new(0.36, 0.7, 0.24), bottom + 0.48, palette::INK),
            part(Vec3::splat(0.24), bottom + 0.98, palette::ACCENT_LIGHT),
        ],
    ));
}

fn drive(
    time: Res<Time>,
    input: Res<SkaterInput>,
    mut skaters: Query<(&Rotation, &mut LinearVelocity, &mut AngularVelocity), With<Skater>>,
) {
    let dt = time.delta_secs();
    for (rotation, mut velocity, mut spin) in &mut skaters {
        let forward = (rotation.0 * Vec3::NEG_Z).with_y(0.0).normalize_or_zero();
        let throttle = if input.throttle >= 0.0 {
            input.throttle * PUSH
        } else {
            input.throttle * BRAKE
        };
        // Reading the speed back from the velocity lets collisions slow the skater down.
        let speed = (velocity.dot(forward) + (throttle - ROLLING_DRAG) * dt).clamp(0.0, TOP_SPEED);
        // Wheels don't slide sideways: keep the forward part and let gravity have the rest.
        velocity.0 = forward * speed + Vec3::Y * velocity.y;
        spin.0 = Vec3::Y * input.steer * TURN_RATE;
    }
}
