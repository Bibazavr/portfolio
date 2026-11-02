//! The plaza: a concrete slab, ledges around the edge and a few obstacles,
//! all made of primitives.

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::palette;

/// Half the side of the square plaza, in metres.
const HALF_SIZE: f32 = 30.0;

pub fn plugin(app: &mut App) {
    app.insert_resource(ClearColor(palette::SKY))
        .insert_resource(GlobalAmbientLight {
            color: Color::WHITE,
            brightness: 700.0,
            ..default()
        })
        .add_systems(Startup, spawn);
}

fn spawn(
    mut commands: Commands,
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
    let accent = matte(palette::ACCENT);
    let accent_light = matte(palette::ACCENT_LIGHT);
    let metal = matte(palette::METAL);

    let mut block = |size: Vec3, position: Vec3, material: &Handle<StandardMaterial>| {
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
        block(size, Vec3::new(x, size.y / 2.0, z), &concrete_dark);
    }

    // Obstacles to skate around.
    block(
        Vec3::new(4.0, 0.5, 1.2),
        Vec3::new(-6.0, 0.25, 6.0),
        &accent,
    );
    block(
        Vec3::new(2.5, 1.0, 2.5),
        Vec3::new(8.0, 0.5, -4.0),
        &accent_light,
    );
    // Three steps; the top one is a deep platform.
    let mut z = -12.0;
    for (height, depth) in [(0.3, 0.8), (0.6, 0.8), (0.9, 3.0)] {
        block(
            Vec3::new(5.0, height, depth),
            Vec3::new(0.0, height / 2.0, z - depth / 2.0),
            &concrete_dark,
        );
        z -= depth;
    }
    // A rail on two posts.
    block(
        Vec3::new(0.08, 0.08, 6.0),
        Vec3::new(-10.0, 0.55, -6.0),
        &metal,
    );
    for z in [-8.5, -3.5] {
        block(
            Vec3::new(0.08, 0.5, 0.08),
            Vec3::new(-10.0, 0.25, z),
            &metal,
        );
    }

    commands.spawn((
        DirectionalLight {
            illuminance: 6_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(10.0, 24.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
