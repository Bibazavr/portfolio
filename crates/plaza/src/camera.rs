//! A third-person camera that trails the skater, and comes round to face
//! a spot with a lot to read while the skater lingers there.

use avian3d::prelude::*;
use bevy::{
    pbr::{DistanceFog, FogFalloff},
    prelude::*,
    render::view::Tonemapping,
    transform::TransformSystems,
    window::PrimaryWindow,
};

use crate::{Layer, palette, skater::Skater, spots::InZone};

/// Where the camera sits relative to the skater: above and behind.
const OFFSET: Vec3 = Vec3::new(0.0, 3.2, 6.5);
const LOOK_AT_HEIGHT: f32 = 1.0;
/// How quickly the camera catches up; higher is snappier.
const DECAY_RATE: f32 = 5.0;
/// With a wall between the skater and the camera, the camera comes this far
/// in front of it, but no closer to the skater than `MIN_DISTANCE`.
const CLEARANCE: f32 = 0.3;
const MIN_DISTANCE: f32 = 1.5;

/// Vertical field of view on landscape screens.
const BASE_FOV: f32 = 45f32.to_radians();
/// On a portrait phone the base view is only about 20° wide, so the vertical
/// field of view grows until the horizontal one reaches this, up to `MAX_FOV`.
const MIN_HORIZONTAL_FOV: f32 = 45f32.to_radians();
const MAX_FOV: f32 = 80f32.to_radians();
/// Screens narrower than this see too little across even then, so the
/// camera also trails farther back, up to `MAX_REACH` times as far.
const NARROW_ASPECT: f32 = 0.75;
const MAX_REACH: f32 = 1.5;
/// How quickly the camera comes round to a `Frame` and back.
const FRAME_RATE: f32 = 3.0;

/// How much farther than `OFFSET` the camera trails.
#[derive(Component)]
pub struct Reach(f32);

/// An area for the camera to face while the skater is in the zone that has
/// it, close enough to read what's on it, following the skater across it.
#[derive(Component, Clone, Copy)]
pub struct Frame {
    /// The middle of the area.
    pub centre: Vec3,
    /// The area's own right and up, as it's read.
    pub across: Dir3,
    pub up: Dir3,
    /// From the area toward where the camera looks at it from.
    pub toward: Dir3,
    /// The area's size, across and up.
    pub size: Vec2,
    /// The least of it to show at once, so that it stays big enough to read.
    pub view: Vec2,
}

impl Frame {
    /// Where the camera goes to show the part of the area around `skater`,
    /// the point it looks at, and which way is up.
    fn shot(&self, lens: &PerspectiveProjection, skater: Vec3) -> (Vec3, Vec3, Vec3) {
        let half_tan = Vec2::new(lens.aspect_ratio, 1.0) * (lens.fov / 2.0).tan();
        let distance = (self.view / 2.0 / half_tan).max_element();
        // Keep the view on the area, if it's bigger than the view.
        let room = (self.size / 2.0 - half_tan * distance).max(Vec2::ZERO);
        let from_centre = skater - self.centre;
        let along = Vec2::new(from_centre.dot(*self.across), from_centre.dot(*self.up));
        let along = along.clamp(-room, room);
        let focus = self.centre + self.across * along.x + self.up * along.y;
        (focus + self.toward * distance, focus, *self.up)
    }
}

/// The last shot of a `Frame`, and how far round to it the camera is, from
/// 0 trailing the skater to 1 facing it.
#[derive(Default)]
pub struct Framing {
    shot: Option<(Vec3, Vec3, Vec3)>,
    blend: f32,
}

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn)
        .add_systems(Update, fit_fov)
        .add_systems(PostUpdate, follow.before(TransformSystems::Propagate));
}

fn fit_fov(
    window: Single<&Window, (With<PrimaryWindow>, Changed<Window>)>,
    camera: Single<(&mut Projection, &mut Reach), With<Camera3d>>,
) {
    let aspect = window.width() / window.height().max(1.0);
    let fov = (2.0 * ((MIN_HORIZONTAL_FOV / 2.0).tan() / aspect).atan()).clamp(BASE_FOV, MAX_FOV);
    let (mut projection, mut reach) = camera.into_inner();
    if let Projection::Perspective(perspective) = projection.as_mut() {
        perspective.fov = fov;
    }
    reach.0 = (NARROW_ASPECT / aspect).clamp(1.0, MAX_REACH);
}

fn spawn(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Reach(1.0),
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

#[allow(clippy::too_many_arguments)]
pub fn follow(
    time: Res<Time>,
    spatial: SpatialQuery,
    bodies: Query<&RigidBody>,
    in_zone: Res<InZone>,
    frames: Query<&Frame>,
    skater: Single<&Transform, (With<Skater>, Without<Camera3d>)>,
    camera: Single<(&mut Transform, &Reach, &Projection), With<Camera3d>>,
    mut framing: Local<Framing>,
) {
    let (mut camera, reach, projection) = camera.into_inner();
    let (yaw, _, _) = skater.rotation.to_euler(EulerRot::YXZ);
    let offset = OFFSET * Vec3::new(1.0, 1.0, reach.0);
    let eye = skater.translation + Vec3::Y * LOOK_AT_HEIGHT;
    let mut target = skater.translation + Quat::from_rotation_y(yaw) * offset;
    // Come in front of a wall rather than lose the skater behind it. Only
    // fixed things count: the skater and the letters move about.
    if let Ok((direction, distance)) = Dir3::new_and_length(target - eye) {
        let filter = SpatialQueryFilter::from_mask(Layer::Default);
        let fixed = |entity| !bodies.get(entity).is_ok_and(RigidBody::is_dynamic);
        if let Some(hit) =
            spatial.cast_ray_predicate(eye, direction, distance, true, &filter, &fixed)
        {
            target = eye + direction * (hit.distance - CLEARANCE).max(MIN_DISTANCE);
        }
    }

    // Come round to a frame, and back to trailing after.
    let frame = in_zone.get().and_then(|zone| frames.get(zone).ok());
    if let (Some(frame), Projection::Perspective(lens)) = (frame, projection) {
        framing.shot = Some(frame.shot(lens, skater.translation));
    }
    let facing = if frame.is_some() { 1.0 } else { 0.0 };
    framing
        .blend
        .smooth_nudge(&facing, FRAME_RATE, time.delta_secs());
    let (mut look, mut up) = (eye, Vec3::Y);
    if let Some((at, focus, frame_up)) = framing.shot {
        let t = framing.blend * framing.blend * (3.0 - 2.0 * framing.blend);
        target = target.lerp(at, t);
        look = eye.lerp(focus, t);
        // Turned to the frame's own up, a view straight down stays square.
        up = up.lerp(frame_up, t);
    }
    camera
        .translation
        .smooth_nudge(&target, DECAY_RATE, time.delta_secs());
    camera.look_at(look, up);
}
