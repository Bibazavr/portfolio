//! The skater: pushes, carves and ollies, rolls up ramps and back down fakie,
//! gets air out of the halfpipe, grinds rails and pumps round the pump track.
//! Arcade handling, tuned to be fun for half a minute rather than realistic.

use std::f32::consts::PI;

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::{
    Layer, bridge,
    input::SkaterInput,
    loading::Phase,
    pump_track::{PumpTrack, Surface},
    spots::{self, Rails},
};

// Metres and seconds.
const TOP_SPEED: f32 = 10.0;
/// Ramps can take the skater past what pushing reaches.
const MAX_SPEED: f32 = 13.0;
const PUSH: f32 = 7.0;
/// After reaching top speed the skater coasts until this much slower, then
/// pushes again.
const COAST: f32 = 1.0;
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
/// Below this, the skater has somehow left the plaza and goes back to the start.
const FELL_OFF: f32 = -5.0;
/// On the pump track, holding the push pumps instead above this speed:
/// speed comes from riding down the far side of each table and roller,
/// `PUMP` times the slope, up to top speed.
const PUMP_FROM: f32 = 3.0;
const PUMP: f32 = 25.0;
/// Going down a slope steeper than this, the rider stands up to press.
const PRESS_SLOPE: f32 = 0.05;
/// How quickly the pump track turns a rider who lets go of the stick to run
/// along it, and how much it turns them back toward its middle line, per
/// metre off it and at most.
const GUIDE_RATE: f32 = 6.0;
const CENTRING: f32 = 0.3;
const MAX_CENTRING: f32 = 0.35;
/// Steering less than this, as a touch stick pushed nearly straight up does,
/// counts as letting go.
const LET_GO: f32 = 0.5;

const RADIUS: f32 = 0.3;
pub const HEIGHT: f32 = 1.2;
const HALF_HEIGHT: f32 = HEIGHT / 2.0;
/// The ground probe's sphere, a little smaller than the capsule's end.
const PROBE_RADIUS: f32 = RADIUS * 0.9;
/// The ground is under the board when the probe meets it within this.
const PROBE_REACH: f32 = 0.15;
/// On the pump track, the board keeps to the surface from this far above
/// it, so it rolls over a table's edge rather than off it.
const SNAP_REACH: f32 = 0.5;

#[derive(Component)]
pub struct Skater;

/// What the board is doing.
#[derive(Component, Default)]
pub struct Board {
    pub motion: Motion,
    /// The surface under the board, or the last one it touched.
    pub normal: Vec3,
    /// Speeding up with a foot on the ground.
    pub pushing: bool,
    pub pump: Pump,
}

/// Pumping the pump track: crouching over the tops, then standing up to
/// press the board down the far side, which is where the speed comes from.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Pump {
    #[default]
    Off,
    Absorb,
    Press,
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
        .add_systems(
            FixedUpdate,
            (skate, back_to_start).run_if(in_state(Phase::Playing)),
        );
}

/// The fences keep the skater on the plaza; if anything still lets them
/// fall through, they start again rather than fall for ever.
fn back_to_start(
    mut skaters: Query<
        (
            &mut Board,
            &mut Position,
            &mut Rotation,
            &mut LinearVelocity,
            &mut GravityScale,
        ),
        With<Skater>,
    >,
) {
    for (mut board, mut position, mut rotation, mut velocity, mut gravity) in &mut skaters {
        if position.y < FELL_OFF {
            *board = Board::default();
            position.0 = spots::START + Vec3::Y * HALF_HEIGHT;
            rotation.0 = Quat::IDENTITY;
            velocity.0 = Vec3::ZERO;
            gravity.0 = 1.0;
        }
    }
}

pub fn spawn(mut commands: Commands) {
    // The ground probe: a sphere at the capsule's lower end, cast down.
    let probe = ShapeCaster::new(
        Collider::sphere(PROBE_RADIUS),
        Vec3::NEG_Y * (HALF_HEIGHT - RADIUS),
        Quat::IDENTITY,
        Dir3::NEG_Y,
    )
    .with_max_distance(SNAP_REACH)
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

#[allow(clippy::too_many_arguments)]
fn skate(
    time: Res<Time>,
    mut input: ResMut<SkaterInput>,
    rails: Res<Rails>,
    track: Option<Res<PumpTrack>>,
    surfaces: Query<(), With<Surface>>,
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
    let floor = hits.iter().find(|hit| hit.normal1.y > 0.3);
    let ground = floor
        .filter(|hit| hit.distance <= PROBE_REACH)
        .map(|hit| hit.normal1);
    // On the pump track, how far the board is above the surface.
    let on_track = floor
        .filter(|hit| surfaces.contains(hit.entity))
        .map(|hit| (hit.normal1, hit.distance));
    let forward = (rotation.0 * Vec3::NEG_Z)
        .with_y(0.0)
        .normalize_or(Vec3::NEG_Z);

    let was_pushing = std::mem::take(&mut board.pushing);
    board.pump = Pump::Off;
    board.motion = match board.motion {
        Motion::Rolling => match ground.or(on_track.map(|(normal, _)| normal)) {
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
                // Over a table's edge, come down onto the surface beyond it.
                if let Some((_, above)) = on_track {
                    let gap = (RADIUS - PROBE_RADIUS) / normal.y;
                    position.y -= (above - gap).max(0.0);
                }
                // Roll along the surface in the direction the board faces.
                // Speed is signed, so going up a ramp and back down is fakie.
                let along = (forward - normal * forward.dot(normal)).normalize_or(forward);
                let mut speed = velocity.dot(along);
                let pumping = on_track.is_some() && input.throttle > 0.0 && speed > PUMP_FROM;
                let downhill = (-along.y).max(0.0);
                if pumping {
                    board.pump = if downhill > PRESS_SLOPE {
                        Pump::Press
                    } else {
                        Pump::Absorb
                    };
                    if speed < TOP_SPEED {
                        speed += input.throttle * PUMP * downhill * dt;
                    }
                }
                board.pushing = !pumping
                    && input.throttle > 0.0
                    && speed < TOP_SPEED
                    && (was_pushing || speed < TOP_SPEED - COAST);
                if board.pushing {
                    speed += input.throttle * PUSH * dt;
                }
                if input.throttle < 0.0 {
                    speed = toward_zero(speed, -input.throttle * BRAKE * dt);
                }
                speed = toward_zero(speed, ROLLING_DRAG * dt).clamp(-MAX_SPEED, MAX_SPEED);
                velocity.0 = along * speed;
                let guided = on_track
                    .zip(track.as_deref())
                    .filter(|_| input.steer.abs() < LET_GO)
                    .and_then(|(_, track)| guide(track, position.0, forward, speed));
                spin.0 = Vec3::Y * guided.unwrap_or(input.steer * TURN_RATE);
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

/// The yaw rate that keeps a board at `at`, facing `forward` and rolling at
/// `speed`, running along the pump track and drifting back to its middle.
/// `None` when the board faces across the track rather than along it.
fn guide(track: &PumpTrack, at: Vec3, forward: Vec3, speed: f32) -> Option<f32> {
    let place = track.place(at.xz())?;
    let heading = forward.xz().normalize_or_zero();
    let ahead = place.direction.dot(heading);
    if ahead.abs() < 0.5 {
        return None;
    }
    // The track's way in the direction the board faces, and the board's
    // nose turned toward the middle line, or its tail when rolling fakie.
    let way = place.direction * ahead.signum();
    let back = (place.out * CENTRING).clamp(-MAX_CENTRING, MAX_CENTRING) * speed.signum();
    let target = (way - place.outward * back).normalize();
    // Positive yaw turns left; anticlockwise round the loop turns left.
    let error = (-heading.perp_dot(target)).atan2(heading.dot(target));
    let follow = speed * ahead.signum() * place.curvature;
    Some(follow + GUIDE_RATE * error)
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
