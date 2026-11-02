//! What the skater looks like: Kenney's skater kid standing sideways on a
//! board, leaning into the surface it rides, with an animation for each move.
//! Kenney's skate animations stand side-on and look along the board, except
//! the push, which faces forward; the kid turns between the two.

use std::{
    f32::consts::{FRAC_PI_2, PI},
    time::Duration,
};

use bevy::{prelude::*, world_serialization::WorldInstanceReady};

use crate::{
    assets::Models,
    loading::Phase,
    skater::{self, Board, HEIGHT, Motion, Skater},
};

/// Kenney's kid is 0.78 m tall; this makes them about 1.2 m.
const SCALE: f32 = 1.5;
/// The deck's top, where the feet go.
const DECK: f32 = 0.16 * SCALE;
/// How quickly the board leans into a new surface; higher is snappier.
const TILT_RATE: f32 = 12.0;
/// How quickly the kid turns forward to push and back; higher is snappier.
const TURN_RATE: f32 = 14.0;
const BLEND: Duration = Duration::from_millis(150);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Pose {
    Stand,
    Push,
    Air,
    Grind,
}

const POSES: [Pose; 4] = [Pose::Stand, Pose::Push, Pose::Air, Pose::Grind];

impl Pose {
    /// Which way the kid faces, turned about Y from facing along the board.
    fn facing(self) -> Quat {
        match self {
            // Kenney's kid faces +Z; the board's nose points −Z.
            Pose::Push => Quat::from_rotation_y(PI),
            // Side-on, so the head, turned to the kid's right, looks ahead.
            _ => Quat::from_rotation_y(-FRAC_PI_2),
        }
    }
}

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

/// The kid's model, which turns to push.
#[derive(Component)]
struct Kid;

/// The pose an animation player is in.
#[derive(Component)]
struct Current(Pose);

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Phase::Playing), spawn.after(skater::spawn))
        .add_systems(
            Update,
            (lean, animate, turn).run_if(in_state(Phase::Playing)),
        );
}

fn spawn(
    mut commands: Commands,
    models: Res<Models>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    skater: Single<Entity, With<Skater>>,
) {
    let clips = POSES.map(|pose| match pose {
        Pose::Stand => models.skate_stand.clone(),
        Pose::Push => models.skate.clone(),
        Pose::Air => models.skate_air.clone(),
        Pose::Grind => models.crouch.clone(),
    });
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
        WorldAssetRoot(models.skateboard.clone()),
        Transform::from_scale(Vec3::splat(SCALE)),
        ChildOf(rig),
    ));
    commands
        .spawn((
            Kid,
            WorldAssetRoot(models.character.clone()),
            Transform::from_xyz(0.0, DECK, 0.0)
                .with_rotation(Pose::Stand.facing())
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
                .play(&mut player, poses.node(Pose::Stand), Duration::ZERO)
                .repeat();
            commands.entity(entity).insert((
                AnimationGraphHandle(poses.graph.clone()),
                transitions,
                Current(Pose::Stand),
            ));
        }
    }
}

fn pose(board: &Board) -> Pose {
    match board.motion {
        Motion::Rolling if board.pushing => Pose::Push,
        Motion::Rolling => Pose::Stand,
        Motion::Grinding(_) => Pose::Grind,
        Motion::Air(_) => Pose::Air,
    }
}

fn animate(
    board: Single<&Board, With<Skater>>,
    poses: Res<Poses>,
    mut players: Query<(
        &mut AnimationPlayer,
        &mut AnimationTransitions,
        &mut Current,
    )>,
) {
    let pose = pose(&board);
    for (mut player, mut transitions, mut current) in &mut players {
        if current.0 != pose {
            current.0 = pose;
            transitions
                .play(&mut player, poses.node(pose), BLEND)
                .repeat();
        }
    }
}

/// Turns the kid forward to push and back side-on after.
fn turn(
    time: Res<Time>,
    board: Single<&Board, With<Skater>>,
    mut kid: Single<&mut Transform, With<Kid>>,
) {
    kid.rotation
        .smooth_nudge(&pose(&board).facing(), TURN_RATE, time.delta_secs());
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
