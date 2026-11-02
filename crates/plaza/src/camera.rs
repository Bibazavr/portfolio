//! A third-person camera that trails the skater.

use bevy::{
    pbr::{DistanceFog, FogFalloff},
    prelude::*,
    render::view::Tonemapping,
    transform::TransformSystems,
    window::PrimaryWindow,
};

use crate::{palette, skater::Skater};

/// Where the camera sits relative to the skater: above and behind.
const OFFSET: Vec3 = Vec3::new(0.0, 3.2, 6.5);
const LOOK_AT_HEIGHT: f32 = 1.0;
/// How quickly the camera catches up; higher is snappier.
const DECAY_RATE: f32 = 5.0;

/// Vertical field of view on landscape screens.
const BASE_FOV: f32 = 45f32.to_radians();
/// On a portrait phone the base view is only about 20° wide, so the vertical
/// field of view grows until the horizontal one reaches this, up to `MAX_FOV`.
const MIN_HORIZONTAL_FOV: f32 = 45f32.to_radians();
const MAX_FOV: f32 = 80f32.to_radians();

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn)
        .add_systems(Update, fit_fov)
        .add_systems(PostUpdate, follow.before(TransformSystems::Propagate));
}

fn fit_fov(
    window: Single<&Window, (With<PrimaryWindow>, Changed<Window>)>,
    mut projection: Single<&mut Projection, With<Camera3d>>,
) {
    let aspect = window.width() / window.height().max(1.0);
    let fov = (2.0 * ((MIN_HORIZONTAL_FOV / 2.0).tan() / aspect).atan()).clamp(BASE_FOV, MAX_FOV);
    if let Projection::Perspective(perspective) = projection.as_mut() {
        perspective.fov = fov;
    }
}

fn spawn(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        // The default tonemapper needs lookup tables we don't ship.
        Tonemapping::AcesFitted,
        // The far side of the plaza fades into the sky.
        DistanceFog {
            color: palette::SKY,
            falloff: FogFalloff::Linear {
                start: 35.0,
                end: 110.0,
            },
            ..default()
        },
        Transform::from_xyz(0.0, 8.0, 26.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn follow(
    time: Res<Time>,
    skater: Single<&Transform, (With<Skater>, Without<Camera3d>)>,
    mut camera: Single<&mut Transform, With<Camera3d>>,
) {
    let (yaw, _, _) = skater.rotation.to_euler(EulerRot::YXZ);
    let target = skater.translation + Quat::from_rotation_y(yaw) * OFFSET;
    camera
        .translation
        .smooth_nudge(&target, DECAY_RATE, time.delta_secs());
    camera.look_at(skater.translation + Vec3::Y * LOOK_AT_HEIGHT, Vec3::Y);
}
