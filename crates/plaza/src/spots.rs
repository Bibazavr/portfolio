//! The portfolio spots: their props, a trigger zone for each, and a purple pad
//! marking where the zone is. Entering a zone shows that spot's panel.
//!
//! The plaza is 60 m square with the skater starting near the south edge,
//! facing north (−Z). The career line runs west to east, oldest to newest,
//! along the north edge. Ramps, rails, steps and the start box are Kenney
//! models; the rest is primitives.

use std::f32::consts::{FRAC_PI_2, PI};

use avian3d::prelude::*;
use bevy::prelude::*;
use content::Spot;

use crate::{Layer, assets, bridge, palette, skater::Skater};

/// Where the skater starts, inside the start ledge's zone.
pub const START: Vec3 = Vec3::new(0.0, 0.0, 20.5);

const CAREER_Z: f32 = -24.0;
/// Room for the career line along X; the stops spread out to fill it.
const CAREER_LENGTH: f32 = 46.0;
const MAX_CAREER_SPACING: f32 = 3.4;

#[derive(Component)]
struct Zone(Spot);

/// The rails the skater can grind, as the line along each one's top.
#[derive(Resource, Default)]
pub struct Rails(pub Vec<Rail>);

pub struct Rail {
    a: Vec3,
    b: Vec3,
}

impl Rail {
    pub fn length(&self) -> f32 {
        self.a.distance(self.b)
    }

    pub fn direction(&self) -> Vec3 {
        (self.b - self.a).normalize()
    }

    /// The point `t` of the way from `a` to `b`.
    pub fn at(&self, t: f32) -> Vec3 {
        self.a.lerp(self.b, t)
    }

    /// How far along the rail the point nearest to `point` is; outside 0..1
    /// when that point is past an end.
    pub fn nearest(&self, point: Vec3) -> f32 {
        let along = self.b - self.a;
        (point - self.a).dot(along) / along.length_squared()
    }
}

pub fn plugin(app: &mut App) {
    app.add_systems(Startup, spawn)
        .add_systems(Update, track_zones);
}

/// Spawns props with shared materials.
struct Builder<'w, 's, 'a> {
    commands: Commands<'w, 's>,
    assets: &'a AssetServer,
    meshes: &'a mut Assets<Mesh>,
    pad: Handle<StandardMaterial>,
}

impl Builder<'_, '_, '_> {
    fn prop(&mut self, name: &str, transform: Transform, collider: Option<ColliderConstructor>) {
        assets::prop(&mut self.commands, self.assets, name, transform, collider);
    }

    /// A static box `size` wide, tall and deep, with its bottom centre at `at`.
    fn block(&mut self, size: Vec3, at: Vec3, material: &Handle<StandardMaterial>) {
        self.commands.spawn((
            Mesh3d(self.meshes.add(Cuboid::from_size(size))),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(at + Vec3::Y * size.y / 2.0),
            RigidBody::Static,
            Collider::cuboid(size.x, size.y, size.z),
        ));
    }

    /// A wedge whose low edge is centred on `foot` and which rises along
    /// `facing` (radians around Y; 0 rises toward +X).
    fn kicker(&mut self, foot: Vec3, facing: f32, size: Vec3, material: &Handle<StandardMaterial>) {
        let profile = Triangle2d::new(Vec2::ZERO, Vec2::new(size.x, 0.0), size.xy());
        let corners: Vec<Vec3> = [-size.z / 2.0, size.z / 2.0]
            .into_iter()
            .flat_map(|z| profile.vertices.map(|v| v.extend(z)))
            .collect();
        self.commands.spawn((
            Mesh3d(self.meshes.add(Extrusion::new(profile, size.z))),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(foot).with_rotation(Quat::from_rotation_y(facing)),
            RigidBody::Static,
            Collider::convex_hull(&corners).expect("a wedge has volume"),
        ));
    }

    /// A trigger zone `size` (x, z) across, centred on `center` (x, z). A pad
    /// marks it on the ground unless the spot is meant to be found.
    fn zone(&mut self, spot: Spot, center: Vec2, size: Vec2, pad: bool) {
        self.commands.spawn((
            Zone(spot),
            Transform::from_xyz(center.x, 1.0, center.y),
            Sensor,
            Collider::cuboid(size.x, 2.0, size.y),
            CollisionEventsEnabled,
            // Only the skater sets zones off, and the ground probe looks past them.
            CollisionLayers::new(Layer::Zone, Layer::Default),
        ));
        if pad {
            let half_size = (size - 0.4) / 2.0;
            self.commands.spawn((
                Mesh3d(self.meshes.add(Plane3d::new(Vec3::Y, half_size))),
                MeshMaterial3d(self.pad.clone()),
                Transform::from_xyz(center.x, 0.01, center.y),
            ));
        }
    }
}

fn spawn(
    commands: Commands,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let portfolio = content::load().expect("content/portfolio.ron is checked by the content tests");

    let mut matte = |color: Color| {
        materials.add(StandardMaterial {
            base_color: color,
            perceptual_roughness: 0.9,
            ..default()
        })
    };
    let accent = matte(palette::ACCENT);
    let concrete_dark = matte(palette::CONCRETE_DARK);
    let ink = matte(palette::INK);
    let metal = matte(palette::METAL);
    let paper = matte(palette::PAPER);
    let pad = materials.add(StandardMaterial {
        base_color: palette::ACCENT.with_alpha(0.3),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    let mut b = Builder {
        commands,
        assets: &assets,
        meshes: &mut meshes,
        pad,
    };
    let hull = Some(ColliderConstructor::ConvexHullFromMesh);

    // Start ledge: who I am. Kenney's box has sloped ends; turned to run east-west.
    let ledge = Transform::from_xyz(0.0, 0.0, 24.5)
        .with_rotation(Quat::from_rotation_y(FRAC_PI_2))
        .with_scale(Vec3::new(1.4, 2.2, 5.0));
    b.prop("obstacle-box", ledge, hull.clone());
    b.zone(
        Spot::Start,
        Vec2::new(START.x, 21.5),
        Vec2::new(8.0, 7.0),
        true,
    );

    // Career line: a kicker per job or project, oldest in the west.
    let stops = portfolio.career().len();
    let spacing = (CAREER_LENGTH / stops as f32).min(MAX_CAREER_SPACING);
    let first = -spacing * (stops - 1) as f32 / 2.0;
    let path = Vec2::new(spacing * stops as f32 + 1.0, 5.0);
    b.commands.spawn((
        Mesh3d(b.meshes.add(Plane3d::new(Vec3::Y, path / 2.0))),
        MeshMaterial3d(concrete_dark.clone()),
        Transform::from_xyz(0.0, 0.005, CAREER_Z),
    ));
    for i in 0..stops {
        let x = first + i as f32 * spacing;
        let kicker = Transform::from_xyz(x, 0.0, CAREER_Z)
            .with_rotation(Quat::from_rotation_y(-FRAC_PI_2))
            .with_scale(Vec3::new(2.2, 1.3, 1.8));
        b.prop("ramp", kicker, hull.clone());
        b.zone(
            Spot::Career(i),
            Vec2::new(x, CAREER_Z),
            Vec2::new(spacing, 4.4),
            true,
        );
    }

    // Big gap: a kicker, a three-metre gap, a platform to land on, a ramp down.
    let gap_z = 4.0;
    b.kicker(
        Vec3::new(8.5, 0.0, gap_z),
        0.0,
        Vec3::new(2.5, 0.8, 3.0),
        &accent,
    );
    b.block(
        Vec3::new(6.0, 0.6, 3.0),
        Vec3::new(17.0, 0.0, gap_z),
        &concrete_dark,
    );
    b.kicker(
        Vec3::new(22.5, 0.0, gap_z),
        PI,
        Vec3::new(2.5, 0.6, 3.0),
        &concrete_dark,
    );
    b.zone(
        Spot::Gap,
        Vec2::new(15.5, gap_z),
        Vec2::new(16.0, 6.0),
        true,
    );

    // Rail garden: a rail per open-source repo. Rails have no colliders: the
    // board locks onto one from above (see `Rails::catch`) and rides through
    // them on the ground rather than snagging.
    let rail_scale = Vec3::new(2.0, 2.0, 8.0);
    let rail_top = 0.25 * rail_scale.y;
    let mut rails = Vec::new();
    for i in 0..portfolio.open_source.len() {
        let x = -25.0 + i as f32 * 2.5;
        let at = Vec3::new(x, 0.0, -12.0);
        b.prop(
            "rail-low",
            Transform::from_translation(at).with_scale(rail_scale),
            None,
        );
        let half = Vec3::Z * rail_scale.z / 2.0;
        rails.push(Rail {
            a: at - half + Vec3::Y * rail_top,
            b: at + half + Vec3::Y * rail_top,
        });
        b.zone(
            Spot::Rail(i),
            Vec2::new(x, -12.0),
            Vec2::new(1.8, 9.0),
            true,
        );
    }
    b.commands.insert_resource(Rails(rails));

    // Skill shop, with a deck leaning on it.
    b.block(Vec3::new(5.0, 3.0, 3.5), Vec3::new(-24.5, 0.0, 3.0), &paper);
    b.block(
        Vec3::new(5.4, 0.15, 1.2),
        Vec3::new(-24.5, 2.6, 5.3),
        &accent,
    );
    b.commands.spawn((
        Mesh3d(b.meshes.add(Cuboid::new(0.9, 3.0, 0.08))),
        MeshMaterial3d(ink.clone()),
        Transform::from_xyz(-23.0, 1.45, 5.15).with_rotation(Quat::from_rotation_x(-0.25)),
    ));
    b.zone(
        Spot::SkillShop,
        Vec2::new(-24.5, 9.0),
        Vec2::new(7.0, 6.0),
        true,
    );

    // Phone booth: contact and CV. Its window faces the middle of the plaza.
    b.block(
        Vec3::new(1.4, 2.6, 1.4),
        Vec3::new(-24.5, 0.0, 20.0),
        &accent,
    );
    b.block(
        Vec3::new(1.44, 1.3, 1.0),
        Vec3::new(-24.5, 0.9, 20.0),
        &paper,
    );
    b.zone(
        Spot::PhoneBooth,
        Vec2::new(-21.5, 20.0),
        Vec2::new(6.0, 6.0),
        true,
    );

    // Billboard: the apps. It faces the start, and the board sits above the
    // trailing camera, so riding between the posts doesn't blank the view.
    const BOARD_BOTTOM: f32 = 4.6;
    for x in [-2.6, 2.6] {
        b.block(
            Vec3::new(0.25, BOARD_BOTTOM, 0.25),
            Vec3::new(x, 0.0, -14.0),
            &metal,
        );
    }
    b.block(
        Vec3::new(6.0, 3.0, 0.3),
        Vec3::new(0.0, BOARD_BOTTOM, -14.0),
        &ink,
    );
    b.block(
        Vec3::new(5.4, 2.4, 0.05),
        Vec3::new(0.0, BOARD_BOTTOM + 0.3, -13.83),
        &accent,
    );
    b.zone(
        Spot::Billboard,
        Vec2::new(0.0, -9.0),
        Vec2::new(8.0, 6.0),
        true,
    );

    // Stair set: Kenney's three steps, rising toward −Z.
    let steps = Transform::from_xyz(20.0, 0.0, -12.25).with_scale(Vec3::new(5.0, 3.0, 4.5));
    b.prop("steps", steps, Some(ColliderConstructor::TrimeshFromMesh));
    b.zone(
        Spot::Stairs,
        Vec2::new(20.0, -8.0),
        Vec2::new(7.0, 5.0),
        true,
    );

    // The Bibacode sticker, hidden in the south-east corner.
    let sticker = Vec3::new(27.4, 0.012, 27.4);
    let turned = Quat::from_rotation_y(0.35);
    b.commands.spawn((
        Mesh3d(b.meshes.add(Cuboid::new(0.7, 0.02, 0.7))),
        MeshMaterial3d(accent),
        Transform::from_translation(sticker).with_rotation(turned),
    ));
    b.commands.spawn((
        Mesh3d(b.meshes.add(Cuboid::new(0.4, 0.02, 0.4))),
        MeshMaterial3d(paper),
        Transform::from_translation(sticker + Vec3::Y * 0.005).with_rotation(turned),
    ));
    b.zone(Spot::Sticker, sticker.xz(), Vec2::splat(3.0), false);
}

/// Keeps the zones the skater is in, most recently entered last, and shows
/// that one's panel. Neighbouring zones touch, so the skater can be in two.
fn track_zones(
    mut started: MessageReader<CollisionStart>,
    mut ended: MessageReader<CollisionEnd>,
    zones: Query<&Zone>,
    skaters: Query<(), With<Skater>>,
    mut inside: Local<Vec<Entity>>,
    mut shown: Local<Option<Spot>>,
) {
    let zone_of = |a: Entity, b: Entity| {
        if zones.contains(a) && skaters.contains(b) {
            Some(a)
        } else if zones.contains(b) && skaters.contains(a) {
            Some(b)
        } else {
            None
        }
    };
    for event in started.read() {
        if let Some(zone) = zone_of(event.collider1, event.collider2) {
            inside.retain(|&z| z != zone);
            inside.push(zone);
        }
    }
    for event in ended.read() {
        if let Some(zone) = zone_of(event.collider1, event.collider2) {
            inside.retain(|&z| z != zone);
        }
    }

    let active = inside
        .last()
        .and_then(|&zone| zones.get(zone).ok())
        .map(|zone| zone.0);
    if active != *shown {
        *shown = active;
        match active {
            Some(spot) => bridge::show(spot),
            None => bridge::hide(),
        }
    }
}
