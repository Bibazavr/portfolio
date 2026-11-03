//! The pump track, which carries the career line: a loop at the plaza's
//! north end with a table per job or project, oldest first, east along the
//! south straight and back west along the north one, and banked berms at
//! both ends. A slot on a straight with no table left for it gets a pair of
//! rollers. It is one mesh built from the layout here, which `skater` also
//! reads to keep a rider who lets go of the stick going round the loop.
//!
//! Positions along the loop are measured anticlockwise seen from above:
//! east along the south straight, round the east berm, west along the north
//! straight and round the west berm. Across it, they are measured outward
//! from the middle line.

use std::f32::consts::PI;

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

/// The middle of the loop, on the plaza (x, z).
const CENTRE: Vec2 = Vec2::new(0.0, -29.5);
/// The berms turn on this radius, at the middle of the track.
const RADIUS: f32 = 6.0;
/// The riding surface, edge to edge.
pub const WIDTH: f32 = 4.0;
/// The sides slope from the riding surface's edges down to the plaza.
const SHOULDER: f32 = 1.2;
/// The riding surface sits this far above the plaza, so the two don't flicker.
const LIFT: f32 = 0.03;
/// Room for the tables along each straight; they spread out to fill it.
const STRAIGHT_LENGTH: f32 = 32.2;
const MAX_PITCH: f32 = 4.6;
const TABLE_HEIGHT: f32 = 0.55;
/// Each table's ramps, up and down, along the track.
const TABLE_RAMP: f32 = 1.3;
/// Flat between one table and the next.
const TABLE_GAP: f32 = 0.6;
/// Flat before the first table and after the last, at the berms' ends.
const RUN_OUT: f32 = 1.5;
const ROLLER_HEIGHT: f32 = 0.4;
/// The berms' outer edge, at their steepest.
const BERM_HEIGHT: f32 = 1.4;
/// The bank rises over this much of each berm's arc, in radians, and falls
/// over as much at its other end.
const BANK_IN: f32 = 0.6;
/// The mesh's resolution: rows this far apart along the track, and this many
/// strips across the riding surface.
const STEP: f32 = 0.1;
const STRIPS: usize = 16;

/// The track's layout, for a number of career stops.
#[derive(Resource)]
pub struct PumpTrack {
    stops: usize,
    /// Places for a table on each straight.
    slots: usize,
    /// From one table's middle to the next.
    pitch: f32,
    /// Half the straights' length.
    half: f32,
}

/// The track's surface, which the skater rides differently.
#[derive(Component)]
pub struct Surface;

/// Where a point on the track is.
pub struct Place {
    /// The way the track runs there, anticlockwise round the loop (x, z).
    pub direction: Vec2,
    /// Away from the loop's inside (x, z).
    pub outward: Vec2,
    /// How far out from the middle line the point is.
    pub out: f32,
    /// How sharply a line through the point alongside the middle one turns
    /// left, going anticlockwise: one over its radius, or zero on a straight.
    pub curvature: f32,
}

impl PumpTrack {
    pub fn new(stops: usize) -> Self {
        let slots = stops.div_ceil(2).max(1);
        let pitch = (STRAIGHT_LENGTH / slots as f32).min(MAX_PITCH);
        Self {
            stops,
            slots,
            pitch,
            half: slots as f32 * pitch / 2.0 + RUN_OUT,
        }
    }

    /// How far along the loop table `i`'s middle is.
    fn along(&self, i: usize) -> f32 {
        let (slot, straight) = if i < self.slots {
            (i, 0.0)
        } else {
            (i - self.slots, self.back())
        };
        straight + RUN_OUT + (slot as f32 + 0.5) * self.pitch
    }

    /// Where the north straight starts along the loop.
    fn back(&self) -> f32 {
        2.0 * self.half + PI * RADIUS
    }

    /// The middle of table `i`'s top, and the way the track runs over it.
    pub fn table(&self, i: usize) -> (Vec3, Vec2) {
        let s = self.along(i);
        let (point, direction, round) = self.line(s);
        let top = Vec3::new(point.x, self.height(s, 0.0, round), point.y);
        (top, direction)
    }

    /// The point in the loop's inside next to table `i` (x, z).
    pub fn beside(&self, i: usize) -> Vec2 {
        let (point, direction, _) = self.line(self.along(i));
        let outward = Vec2::new(-direction.y, direction.x);
        let inside = RADIUS - self.full_width() / 2.0;
        point - outward * (RADIUS - inside / 2.0)
    }

    /// A table's flat top, along the track.
    pub fn table_top(&self) -> f32 {
        self.pitch - 2.0 * TABLE_RAMP - TABLE_GAP
    }

    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    /// From one edge of the track's sides to the other.
    pub fn full_width(&self) -> f32 {
        WIDTH + 2.0 * SHOULDER
    }

    fn length(&self) -> f32 {
        2.0 * self.back()
    }

    /// The point on the middle line `s` along the loop, the way it runs
    /// there, and how far round its berm it is (zero on the straights).
    fn line(&self, s: f32) -> (Vec2, Vec2, Option<f32>) {
        let straight = 2.0 * self.half;
        let berm = PI * RADIUS;
        let ends = [Vec2::new(self.half, 0.0), Vec2::new(-self.half, 0.0)];
        // The two halves of the loop mirror each other through its centre.
        let (s, flip) = if s < straight + berm {
            (s, 1.0)
        } else {
            (s - straight - berm, -1.0)
        };
        let (point, direction, round) = if s < straight {
            (Vec2::new(s - self.half, RADIUS), Vec2::X, None)
        } else {
            let angle = (s - straight) / RADIUS;
            let (sin, cos) = angle.sin_cos();
            (
                ends[0] + RADIUS * Vec2::new(sin, cos),
                Vec2::new(cos, -sin),
                Some(angle),
            )
        };
        (CENTRE + point * flip, direction * flip, round)
    }

    /// Height of the riding surface's middle `s` along the loop, from the
    /// tables and rollers.
    fn base(&self, s: f32) -> f32 {
        let straight = 2.0 * self.half;
        let (x, first) = if s < straight {
            (s, 0)
        } else if (self.back()..self.back() + straight).contains(&s) {
            (s - self.back(), self.slots)
        } else {
            return 0.0;
        };
        let x = x - RUN_OUT;
        let slot = (x / self.pitch).floor();
        if !(0.0..self.slots as f32).contains(&slot) {
            return 0.0;
        }
        let from_middle = (x - (slot + 0.5) * self.pitch).abs();
        if first + (slot as usize) < self.stops {
            let ramp = (self.table_top() / 2.0 + TABLE_RAMP - from_middle) / TABLE_RAMP;
            TABLE_HEIGHT * ease(ramp)
        } else {
            ROLLER_HEIGHT * (1.0 - (4.0 * PI * from_middle / self.pitch).cos()) / 2.0
        }
    }

    /// How far the berm `round` radians round has banked, from 0 to 1.
    fn bank(round: Option<f32>) -> f32 {
        round.map_or(0.0, |angle| {
            ease(angle / BANK_IN) * ease((PI - angle) / BANK_IN)
        })
    }

    /// The surface's height `s` along the loop and `out` from the middle line.
    fn height(&self, s: f32, out: f32, round: Option<f32>) -> f32 {
        let edge = WIDTH / 2.0;
        let across = (out.clamp(-edge, edge) + edge) / WIDTH;
        let top = LIFT + self.base(s) + Self::bank(round) * BERM_HEIGHT * across * across;
        let beyond = (out.abs() - edge).max(0.0);
        top * (1.0 - beyond / SHOULDER).max(0.0)
    }

    /// The track as a mesh, its sides meeting the plaza, coloured for a
    /// white material: the riding surface `surface`, the sides `sides`.
    pub fn mesh(&self, surface: Color, sides: Color) -> Mesh {
        let rows = (self.length() / STEP).ceil() as usize;
        let edge = WIDTH / 2.0;
        // A hair outside each edge, so the sides' colour starts sharply there.
        let crease = edge + 0.01;
        let columns: Vec<f32> = [-edge - SHOULDER, -crease]
            .into_iter()
            .chain((0..=STRIPS).map(|j| -edge + j as f32 * WIDTH / STRIPS as f32))
            .chain([crease, edge + SHOULDER])
            .collect();
        let [surface, sides] = [surface, sides].map(|c| c.to_linear().to_f32_array());
        let mut positions = Vec::with_capacity(rows * columns.len());
        let mut colors = Vec::with_capacity(rows * columns.len());
        for row in 0..rows {
            let s = row as f32 * self.length() / rows as f32;
            let (point, direction, round) = self.line(s);
            let outward = Vec2::new(-direction.y, direction.x);
            for &out in &columns {
                let at = point + outward * out;
                positions.push([at.x, self.height(s, out, round), at.y]);
                colors.push(if out.abs() > edge { sides } else { surface });
            }
        }
        // Quads between neighbouring rows, the last row joining the first.
        let width = columns.len() as u32;
        let mut indices = Vec::with_capacity(rows * (columns.len() - 1) * 6);
        for row in 0..rows as u32 {
            let next = (row + 1) % rows as u32;
            for j in 0..width - 1 {
                let [a, b] = [row * width + j, next * width + j];
                indices.extend([a, a + 1, b, b, a + 1, b + 1]);
            }
        }
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
        .with_inserted_indices(Indices::U32(indices));
        mesh.compute_smooth_normals();
        mesh
    }

    /// Where `at` (x, z) is on the track, if it's on it.
    pub fn place(&self, at: Vec2) -> Option<Place> {
        let local = at - CENTRE;
        let place = if local.x.abs() <= self.half {
            // On a straight: the south one runs east, the north one west.
            let side = if local.y >= 0.0 { 1.0 } else { -1.0 };
            Place {
                direction: Vec2::X * side,
                outward: Vec2::Y * side,
                out: local.y * side - RADIUS,
                curvature: 0.0,
            }
        } else {
            let from_middle = local - Vec2::new(self.half.copysign(local.x), 0.0);
            let radius = from_middle.length();
            let outward = from_middle.try_normalize()?;
            Place {
                direction: Vec2::new(outward.y, -outward.x),
                outward,
                out: radius - RADIUS,
                curvature: radius.recip(),
            }
        };
        (place.out.abs() <= WIDTH / 2.0 + SHOULDER).then_some(place)
    }
}

/// Eases from 0 to 1 as `t` goes from 0 to 1, flat at both ends.
fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plaza;

    /// The middle line joins up with itself all the way round.
    #[test]
    fn line_is_continuous() {
        let track = PumpTrack::new(13);
        let samples = 2000;
        let step = track.length() / samples as f32;
        for i in 0..samples {
            let (a, _, _) = track.line(i as f32 * step);
            let (b, _, _) = track.line(((i + 1) % samples) as f32 * step);
            assert!(a.distance(b) < step * 1.01, "jump after sample {i}");
        }
    }

    /// `place` finds the middle line where `line` put it.
    #[test]
    fn place_matches_line() {
        let track = PumpTrack::new(13);
        for i in 0..200 {
            let s = i as f32 * track.length() / 200.0;
            let (point, direction, _) = track.line(s);
            let outward = Vec2::new(-direction.y, direction.x);
            let place = track.place(point + outward * 0.5).expect("on the track");
            assert!((place.out - 0.5).abs() < 1e-3, "out {} at s {s}", place.out);
            assert!(place.direction.dot(direction) > 0.999, "direction at s {s}");
        }
        assert!(track.place(CENTRE).is_none());
    }

    /// The loop fits inside the plaza's edge ledges, a metre thick, with
    /// the sides on.
    #[test]
    fn fits_the_plaza() {
        let track = PumpTrack::new(13);
        let across = RADIUS + track.full_width() / 2.0;
        assert!(CENTRE.x + track.half + across < plaza::EAST - 0.5);
        assert!(CENTRE.x - track.half - across > plaza::WEST + 0.5);
        assert!(CENTRE.y - across > plaza::NORTH + 0.5);
    }

    /// Each table stands on a straight, the first half going east and the
    /// rest coming back west, with a flat top long enough to write on.
    #[test]
    fn tables_go_round_the_loop() {
        let track = PumpTrack::new(13);
        assert!(track.table_top() >= 1.2);
        for i in 0..13 {
            let (top, direction) = track.table(i);
            let place = track.place(top.xz()).expect("on the track");
            assert!(place.out.abs() < 1e-3 && place.curvature == 0.0);
            assert!((top.y - LIFT - TABLE_HEIGHT).abs() < 1e-4, "table {i} top");
            let east = if i < 7 { 1.0 } else { -1.0 };
            assert_eq!(direction, Vec2::X * east);
            let infield = track.beside(i);
            assert!(
                track.place(infield).is_none(),
                "beside {i} is off the track"
            );
        }
    }
}
