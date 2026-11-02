//! The plaza itself: a concrete slab, ledges around the edge, the halfpipe,
//! benches and a parked car, under a low afternoon sun. The portfolio spots
//! live in `spots`.

use std::f32::consts::{FRAC_PI_2, PI};

use avian3d::prelude::*;
use bevy::{light::CascadeShadowConfigBuilder, prelude::*};

use crate::{assets, palette};

/// Half the side of the square plaza, in metres.
const HALF_SIZE: f32 = 30.0;
/// Kenney's halfpipe pieces are one metre across and 0.8 m tall; at this size
/// the walls are 2 m high, which a pushed skater clears with air to spare.
const HALFPIPE_SCALE: f32 = 2.5;

pub fn plugin(app: &mut App) {
    app.insert_resource(ClearColor(palette::SKY))
        .insert_resource(GlobalAmbientLight {
            color: palette::SKY,
            brightness: 300.0,
            ..default()
        })
        .add_systems(Startup, spawn);
}

fn spawn(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut matte = |color: Color| {
        materials.add(StandardMaterial {
            base_color: color,
            perceptual_roughness: 0.9,
            ..default()
        })
    };
    let concrete = matte(palette::CONCRETE);
    let concrete_dark = matte(palette::CONCRETE_DARK);

    let mut block = |commands: &mut Commands,
                     size: Vec3,
                     position: Vec3,
                     material: &Handle<StandardMaterial>| {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(size))),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(position),
            RigidBody::Static,
            Collider::cuboid(size.x, size.y, size.z),
        ));
    };

    // A thick ground slab, so a fast body can't tunnel through it.
    let side = HALF_SIZE * 2.0;
    block(
        &mut commands,
        Vec3::new(side, 1.0, side),
        Vec3::new(0.0, -0.5, 0.0),
        &concrete,
    );

    // Ledges along the edge keep the skater on the plaza.
    let edge = Vec3::new(side + 1.0, 0.6, 1.0);
    for (size, x, z) in [
        (edge, 0.0, -HALF_SIZE),
        (edge, 0.0, HALF_SIZE),
        (edge.zyx(), -HALF_SIZE, 0.0),
        (edge.zyx(), HALF_SIZE, 0.0),
    ] {
        block(
            &mut commands,
            size,
            Vec3::new(x, size.y / 2.0, z),
            &concrete_dark,
        );
    }

    // The halfpipe: two quarter pipes, three pieces wide, facing across a flat.
    // It opens to the west, so the way in is along the flat, carving up a wall.
    // Each piece rises toward −Z; the southern pair is turned around.
    let scale = Vec3::splat(HALFPIPE_SCALE);
    let (x, z, flat) = (18.0, 16.0, 4.0);
    for side in [-1.0, 1.0] {
        for column in [-1.0, 0.0, 1.0] {
            let at = Vec3::new(
                x + column * HALFPIPE_SCALE,
                0.0,
                z + side * (flat + HALFPIPE_SCALE) / 2.0,
            );
            let facing = if side < 0.0 { 0.0 } else { PI };
            let transform = Transform::from_translation(at)
                .with_rotation(Quat::from_rotation_y(facing))
                .with_scale(scale);
            assets::prop(
                &mut commands,
                &assets,
                "half-pipe",
                transform,
                Some(ColliderConstructor::TrimeshFromMesh),
            );
        }
    }

    // Benches along the west edge and a car parked by the east edge.
    for z in [-4.0, 12.0] {
        let transform = Transform::from_xyz(-28.3, 0.0, z)
            .with_rotation(Quat::from_rotation_y(FRAC_PI_2))
            .with_scale(Vec3::splat(2.0));
        assets::prop(
            &mut commands,
            &assets,
            "bench",
            transform,
            Some(ColliderConstructor::ConvexHullFromMesh),
        );
    }
    let car_scale = 2.4;
    let car = Transform::from_xyz(27.0, 0.183 * car_scale, -3.0)
        .with_rotation(Quat::from_rotation_y(FRAC_PI_2 + 0.15))
        .with_scale(Vec3::splat(car_scale));
    assets::prop(
        &mut commands,
        &assets,
        "vehicle",
        car,
        Some(ColliderConstructor::ConvexHullFromMesh),
    );

    // A low sun from the south-west, so shadows fall long across the plaza.
    commands.spawn((
        DirectionalLight {
            illuminance: 5_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(-12.0, 18.0, 14.0).looking_at(Vec3::ZERO, Vec3::Y),
        // WebGL2 supports one cascade; keep shadows to what the camera sees up close.
        CascadeShadowConfigBuilder {
            num_cascades: 1,
            maximum_distance: 40.0,
            ..default()
        }
        .build(),
    ));
}
