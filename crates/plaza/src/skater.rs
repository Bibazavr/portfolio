//! The skater: pushes, carves and ollies, rolls up ramps and back down fakie,
//! gets air out of the halfpipe and grinds rails. Arcade handling, tuned to be
//! fun for half a minute rather than realistic.

use std::f32::consts::PI;

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::{
    Layer, bridge,
    input::SkaterInput,
    loading::Phase,
    spots::{self, Rails},
};

// Metres and seconds.
const TOP_SPEED: f32 = 10.0;
/// Ramps can take the skater past what pushing reaches.
const MAX_SPEED: f32 = 13.0;
const PUSH: f32 = 7.0;
const BRAKE: f32 = 12.0;
const ROLLING_DRAG: f32 = 0.5;
const TURN_RATE: f32 = 2.6;
const AIR_TURN_RATE: f32 = 2.0;
const OLLIE_SPEED: f32 = 4.6;
/// After an ollie the ground is ignored this long, so the jump can leave it.
const TAKEOFF_TIME: f32 = 0.12;
/// How close the board has to come to a rail, sideways, to lock onto it.
const GRIND_REACH: f32 = 0.5;
const MIN_GRIND_SPEED: f32 = 3.0;
/// A landing this far from the takeoff counts as a gap.
const GAP_DISTANCE: f32 = 3.5;

const RADIUS: f32 = 0.3;
pub const HEIGHT: f32 = 1.2;
const HALF_HEIGHT: f32 = HEIGHT / 2.0;

#[derive(Component)]
pub struct Skater;

/// What the board is doing.
#[derive(Component, Default)]
pub struct Board {
    pub motion: Motion,
    /// The surface under the board, or the last one it touched.
    pub normal: Vec3,
}

#[derive(Clone, Copy, Default)]
pub enum Motion {
    #[default]
    Rolling,
    Air(Air),
    Grinding(Grind),
}

#[derive(Clone, Copy)]
pub struct Air {
    started: f32,
    from: Vec3,
    ollie: bool,
    /// Off a halfpipe wall: the board turns around in the air.
    halfpipe: bool,
    spin_left: f32,
    spin_rate: f32,
}

impl Air {
    fn new(now: f32, from: Vec3, ollie: bool, surface: Vec3, velocity: Vec3) -> Self {
        let halfpipe = surface.y < 0.5 && velocity.y > 1.5;
        let hang_time = 2.0 * velocity.y.max(0.0) / 9.81;
        Self {
            started: now,
            from,
            ollie,
            halfpipe,
            spin_left: if halfpipe { PI } else { 0.0 },
            spin_rate: PI / hang_time.max(0.35),
        }
    }

    /// The trick, if the air was worth one.
    fn landed(&self, now: f32, at: Vec3) -> Option<Trick> {
        let hang = now - self.started;
        let distance = (at - self.from).with_y(0.0).length();
        let (kind, base) = if self.halfpipe {
            (TrickKind::Air, 50)
        } else if distance > GAP_DISTANCE && hang > 0.4 {
            (TrickKind::Gap, 150)
        } else if self.ollie {
            (TrickKind::Ollie, 10)
        } else if hang > 0.5 {
            (TrickKind::Air, 30)
        } else {
            return None;
        };
        Some(Trick {
            kind,
            points: base + (hang * 100.0) as u32,
        })
    }
}

#[derive(Clone, Copy)]
pub struct Grind {
    rail: usize,
    /// How far along the rail, from `a` (0) to `b` (1).
    t: f32,
    /// Signed: positive runs toward `b`.
    speed: f32,
    started: f32,
}

impl Grind {
    fn trick(&self, now: f32) -> Trick {
        Trick {
            kind: TrickKind::Grind,
            points: 10 + ((now - self.started) * 60.0) as u32,
        }
    }
}

/// A finished trick; `score` adds it up.
#[derive(Message, Clone, Copy)]
pub struct Trick {
    pub kind: TrickKind,
    pub points: u32,
}

#[derive(Clone, Copy)]
pub enum TrickKind {
    Ollie,
    Air,
    Grind,
    Gap,
}

impl TrickKind {
    /// Matches the trick names the page holds.
    pub fn id(self) -> &'static str {
        match self {
            TrickKind::Ollie => "ollie",
            TrickKind::Air => "air",
            TrickKind::Grind => "grind",
            TrickKind::Gap => "gap",
        }
    }
}

pub fn plugin(app: &mut App) {
    app.add_message::<Trick>()
        .add_systems(OnEnter(Phase::Playing), spawn)
        .add_systems(FixedUpdate, skate.run_if(in_state(Phase::Playing)));
}

pub fn spawn(mut commands: Commands) {
    // The ground probe: a sphere at the capsule's lower end, cast down.
    let probe = ShapeCaster::new(
        Collider::sphere(RADIUS * 0.9),
        Vec3::NEG_Y * (HALF_HEIGHT - RADIUS),
        Quat::IDENTITY,
        Dir3::NEG_Y,
    )
    .with_max_distance(0.15)
    .with_query_filter(SpatialQueryFilter::from_mask(Layer::Default));

    commands.spawn((
        Skater,
        Board::default(),
        Transform::from_translation(spots::START + Vec3::Y * HALF_HEIGHT),
        Visibility::default(),
        RigidBody::Dynamic,
        // A capsule rides over curves and seams that would snag a box.
        Collider::capsule(RADIUS, HEIGHT - 2.0 * RADIUS),
        // Steering sets the yaw rate; nothing should tip the skater over.
        LockedAxes::new().lock_rotation_x().lock_rotation_z(),
        // `skate` owns the velocity, so contacts shouldn't add drag or bounce.
        Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
        Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
        GravityScale(1.0),
        probe,
        TransformInterpolation,
    ));
}

/// What `skate` reads and steers on the skater.
type Body = (
    &'static mut Board,
    &'static mut Position,
    &'static mut Rotation,
    &'static mut LinearVelocity,
    &'static mut AngularVelocity,
    &'static mut GravityScale,
    &'static ShapeHits,
);

fn skate(
    time: Res<Time>,
    mut input: ResMut<SkaterInput>,
    rails: Res<Rails>,
    mut tricks: MessageWriter<Trick>,
    mut skaters: Query<Body, With<Skater>>,
) {
    let Ok((mut board, mut position, mut rotation, mut velocity, mut spin, mut gravity, hits)) =
        skaters.single_mut()
    else {
        return;
    };
    let dt = time.delta_secs();
    let now = time.elapsed_secs();
    let ground = hits
        .iter()
        .find(|hit| hit.normal1.y > 0.3)
        .map(|hit| hit.normal1);
    let forward = (rotation.0 * Vec3::NEG_Z)
        .with_y(0.0)
        .normalize_or(Vec3::NEG_Z);

    board.motion = match board.motion {
        Motion::Rolling => match ground {
            // Rolled off an edge or over a lip.
            None => Motion::Air(Air::new(now, position.0, false, board.normal, velocity.0)),
            Some(normal) if input.ollie.take().is_some() => {
                board.normal = normal;
                velocity.0 += normal * OLLIE_SPEED;
                bridge::sound("ollie");
                Motion::Air(Air::new(now, position.0, true, normal, velocity.0))
            }
            Some(normal) => {
                board.normal = normal;
                // Roll along the surface in the direction the board faces.
                // Speed is signed, so going up a ramp and back down is fakie.
                let along = (forward - normal * forward.dot(normal)).normalize_or(forward);
                let mut speed = velocity.dot(along);
                if input.throttle > 0.0 && speed < TOP_SPEED {
                    speed += input.throttle * PUSH * dt;
                }
                if input.throttle < 0.0 {
                    speed = toward_zero(speed, -input.throttle * BRAKE * dt);
                }
                speed = toward_zero(speed, ROLLING_DRAG * dt).clamp(-MAX_SPEED, MAX_SPEED);
                velocity.0 = along * speed;
                spin.0 = Vec3::Y * input.steer * TURN_RATE;
                Motion::Rolling
            }
        },

        Motion::Air(mut air) => {
            let mut yaw_rate = input.steer * AIR_TURN_RATE;
            // A halfpipe air turns around, so the landing faces down the wall.
            if air.spin_left > 0.0 {
                let step = (air.spin_rate * dt).min(air.spin_left);
                air.spin_left -= step;
                yaw_rate += step / dt;
            }
            spin.0 = Vec3::Y * yaw_rate;

            let feet = position.0 - Vec3::Y * HALF_HEIGHT;
            if let Some((rail, t)) = rails.catch(feet, velocity.0) {
                let direction = rails.0[rail].direction();
                let mut speed = velocity.dot(direction);
                if speed.abs() < MIN_GRIND_SPEED {
                    let heading = if speed != 0.0 {
                        speed
                    } else {
                        forward.dot(direction)
                    };
                    speed = MIN_GRIND_SPEED.copysign(heading);
                }
                gravity.0 = 0.0;
                Motion::Grinding(Grind {
                    rail,
                    t,
                    speed,
                    started: now,
                })
            } else if let Some(normal) = ground
                && now - air.started > TAKEOFF_TIME
            {
                board.normal = normal;
                bridge::sound("land");
                if let Some(trick) = air.landed(now, position.0) {
                    tricks.write(trick);
                }
                Motion::Rolling
            } else {
                Motion::Air(air)
            }
        }

        Motion::Grinding(mut grind) => {
            let rail = &rails.0[grind.rail];
            let direction = rail.direction();
            grind.t += grind.speed * dt / rail.length();
            let off_the_end = !(0.0..=1.0).contains(&grind.t);
            if off_the_end || input.ollie.take().is_some() {
                velocity.0 = direction * grind.speed;
                let ollie = !off_the_end;
                if ollie {
                    velocity.y = OLLIE_SPEED;
                    bridge::sound("ollie");
                }
                gravity.0 = 1.0;
                tricks.write(grind.trick(now));
                Motion::Air(Air::new(now, position.0, ollie, Vec3::Y, velocity.0))
            } else {
                // Locked to the rail: placed on it each step, facing the way it slides.
                position.0 = rail.at(grind.t) + Vec3::Y * HALF_HEIGHT;
                velocity.0 = direction * grind.speed;
                let facing = (direction * grind.speed.signum()).with_y(0.0).normalize();
                rotation.0 = Quat::from_rotation_arc(Vec3::NEG_Z, facing);
                spin.0 = Vec3::ZERO;
                Motion::Grinding(grind)
            }
        }
    };
}

fn toward_zero(value: f32, amount: f32) -> f32 {
    value.signum() * (value.abs() - amount).max(0.0)
}

impl Rails {
    /// The rail the board's underside at `feet` can lock onto while moving
    /// with `velocity`, and how far along that rail it is.
    fn catch(&self, feet: Vec3, velocity: Vec3) -> Option<(usize, f32)> {
        if velocity.y > 1.0 {
            return None;
        }
        self.0.iter().enumerate().find_map(|(index, rail)| {
            let t = rail.nearest(feet);
            let point = rail.at(t);
            let sideways = (feet - point).with_y(0.0).length();
            let above = feet.y - point.y;
            let on = (0.02..0.98).contains(&t)
                && sideways < GRIND_REACH
                && (-0.25..0.5).contains(&above);
            on.then_some((index, t))
        })
    }
}
