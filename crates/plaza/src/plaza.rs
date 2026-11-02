//! The plaza itself: a concrete slab, ledges around the edge, sky and sun.
//! The spots on it live in `spots`.

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

    commands.spawn((
        DirectionalLight {
            illuminance: 6_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(10.0, 24.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
