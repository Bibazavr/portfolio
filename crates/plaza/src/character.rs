//! What the skater looks like: Kenney's skater kid standing sideways on a
//! board, leaning into the surface it rides, with an animation for each move.

use std::{f32::consts::FRAC_PI_2, time::Duration};

use avian3d::prelude::*;
use bevy::{prelude::*, world_serialization::WorldInstanceReady};

use crate::{
    assets,
    skater::{Board, HEIGHT, Motion, Skater},
};

/// Kenney's kid is 0.78 m tall; this makes them about 1.2 m.
const SCALE: f32 = 1.5;
/// The deck's top, where the feet go.
const DECK: f32 = 0.16 * SCALE;
/// How quickly the board leans into a new surface; higher is snappier.
const TILT_RATE: f32 = 12.0;
const BLEND: Duration = Duration::from_millis(150);

/// Animations in the character's file, by their index in the pack.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pose {
    Idle = 1,
    Jump = 4,
    Fall = 5,
    Crouch = 6,
}

const POSES: [Pose; 4] = [Pose::Idle, Pose::Jump, Pose::Fall, Pose::Crouch];

#[derive(Resource)]
struct Poses {
    graph: Handle<AnimationGraph>,
    nodes: Vec<AnimationNodeIndex>,
}

impl Poses {
    fn node(&self, pose: Pose) -> AnimationNodeIndex {
        let index = POSES
            .iter()
            .position(|&p| p == pose)
            .expect("every pose is loaded");
        self.nodes[index]
    }
}

/// The model's root under the skater, which leans with the surface.
#[derive(Component)]
struct Rig;

/// The pose an animation player is in.
#[derive(Component)]
struct Current(Pose);

pub fn plugin(app: &mut App) {
    app.add_systems(PostStartup, spawn)
        .add_systems(Update, (lean, animate));
}

fn spawn(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    skater: Single<Entity, With<Skater>>,
) {
    let file = assets::model("character-skate-boy");
    let clips = POSES
        .map(|pose| assets.load(GltfAssetLabel::Animation(pose as usize).from_asset(file.clone())));
    let (graph, nodes) = AnimationGraph::from_clips(clips);
    commands.insert_resource(Poses {
        graph: graphs.add(graph),
        nodes,
    });

    let rig = commands
        .spawn((
            Rig,
            Transform::from_xyz(0.0, -HEIGHT / 2.0, 0.0),
            Visibility::default(),
            ChildOf(*skater),
        ))
        .id();
    commands.spawn((
        WorldAssetRoot(assets::scene(&assets, "skateboard")),
        Transform::from_scale(Vec3::splat(SCALE)),
        ChildOf(rig),
    ));
    // Side-on to the direction of travel, like a skater stands.
    commands
        .spawn((
            WorldAssetRoot(assets::scene(&assets, "character-skate-boy")),
            Transform::from_xyz(0.0, DECK, 0.0)
                .with_rotation(Quat::from_rotation_y(FRAC_PI_2))
                .with_scale(Vec3::splat(SCALE)),
            ChildOf(rig),
        ))
        .observe(start_animation);
}

fn start_animation(
    ready: On<WorldInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    mut players: Query<&mut AnimationPlayer>,
    poses: Res<Poses>,
) {
    for entity in children.iter_descendants(ready.entity) {
        if let Ok(mut player) = players.get_mut(entity) {
            let mut transitions = AnimationTransitions::new();
            transitions
                .play(&mut player, poses.node(Pose::Idle), Duration::ZERO)
                .repeat();
            commands.entity(entity).insert((
                AnimationGraphHandle(poses.graph.clone()),
                transitions,
                Current(Pose::Idle),
            ));
        }
    }
}

fn animate(
    skater: Single<(&Board, &LinearVelocity), With<Skater>>,
    poses: Res<Poses>,
    mut players: Query<(
        &mut AnimationPlayer,
        &mut AnimationTransitions,
        &mut Current,
    )>,
) {
    let (board, velocity) = *skater;
    let pose = match board.motion {
        Motion::Rolling => Pose::Idle,
        Motion::Grinding(_) => Pose::Crouch,
        Motion::Air(_) if velocity.y > 0.5 => Pose::Jump,
        Motion::Air(_) => Pose::Fall,
    };
    for (mut player, mut transitions, mut current) in &mut players {
        if current.0 != pose {
            current.0 = pose;
            let animation = transitions.play(&mut player, poses.node(pose), BLEND);
            if pose != Pose::Jump {
                animation.repeat();
            }
        }
    }
}

/// Leans the model to the surface while rolling, and levels it in the air.
fn lean(
    time: Res<Time>,
    skater: Single<(&Board, &Transform), With<Skater>>,
    mut rig: Single<&mut Transform, (With<Rig>, Without<Skater>)>,
) {
    let (board, transform) = *skater;
    let up = match board.motion {
        Motion::Rolling => board.normal,
        _ => Vec3::Y,
    };
    let local_up = (transform.rotation.inverse() * up).normalize_or(Vec3::Y);
    let target = Quat::from_rotation_arc(Vec3::Y, local_up);
    rig.rotation
        .smooth_nudge(&target, TILT_RATE, time.delta_secs());
}
