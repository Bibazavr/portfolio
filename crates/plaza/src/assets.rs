//! Models from Kenney's "Mini Skate" pack (CC0, see assets/models/LICENSE-kenney-mini-skate.txt),
//! with the shared palette texture recoloured to the studio's purple. They are
//! embedded in the binary: a few hundred kilobytes that load with the WASM,
//! need no extra requests and can't go stale next to it.

use std::path::{Path, PathBuf};

use avian3d::prelude::*;
use bevy::{asset::io::embedded::EmbeddedAssetRegistry, prelude::*};
use bevy_asset_loader::prelude::*;

macro_rules! embed {
    ($registry:expr, $($file:literal),* $(,)?) => {
        $(
            $registry.insert_asset(
                PathBuf::new(),
                Path::new($file),
                include_bytes!(concat!("../assets/", $file)).as_slice(),
            );
        )*
    };
}

pub fn plugin(app: &mut App) {
    let registry = app.world().resource::<EmbeddedAssetRegistry>();
    embed!(
        registry,
        "models/Textures/colormap.png",
        "models/bench.glb",
        "models/character-skate-boy.glb",
        "models/half-pipe.glb",
        "models/obstacle-box.glb",
        "models/rail-low.glb",
        "models/ramp.glb",
        "models/skateboard.glb",
        "models/steps.glb",
        "models/vehicle.glb",
    );
}

/// The models, loaded by bevy_asset_loader before the plaza is built (see `loading`).
#[derive(AssetCollection, Resource)]
pub struct Models {
    #[asset(path = "embedded://models/bench.glb#Scene0")]
    pub bench: Handle<WorldAsset>,
    #[asset(path = "embedded://models/character-skate-boy.glb#Scene0")]
    pub character: Handle<WorldAsset>,
    #[asset(path = "embedded://models/half-pipe.glb#Scene0")]
    pub half_pipe: Handle<WorldAsset>,
    #[asset(path = "embedded://models/obstacle-box.glb#Scene0")]
    pub obstacle_box: Handle<WorldAsset>,
    #[asset(path = "embedded://models/rail-low.glb#Scene0")]
    pub rail_low: Handle<WorldAsset>,
    #[asset(path = "embedded://models/ramp.glb#Scene0")]
    pub ramp: Handle<WorldAsset>,
    #[asset(path = "embedded://models/skateboard.glb#Scene0")]
    pub skateboard: Handle<WorldAsset>,
    #[asset(path = "embedded://models/steps.glb#Scene0")]
    pub steps: Handle<WorldAsset>,
    #[asset(path = "embedded://models/vehicle.glb#Scene0")]
    pub vehicle: Handle<WorldAsset>,
    // The character's animations, by their index in the pack.
    #[asset(path = "embedded://models/character-skate-boy.glb#Animation1")]
    pub idle: Handle<AnimationClip>,
    #[asset(path = "embedded://models/character-skate-boy.glb#Animation4")]
    pub jump: Handle<AnimationClip>,
    #[asset(path = "embedded://models/character-skate-boy.glb#Animation5")]
    pub fall: Handle<AnimationClip>,
    #[asset(path = "embedded://models/character-skate-boy.glb#Animation6")]
    pub crouch: Handle<AnimationClip>,
}

/// Spawns a model that doesn't move. With a `collider`, colliders are built
/// from its meshes once it spawns: a convex hull for simple solid shapes, a
/// triangle mesh for curved or stepped ones.
pub fn prop(
    commands: &mut Commands,
    scene: &Handle<WorldAsset>,
    transform: Transform,
    collider: Option<ColliderConstructor>,
) {
    let mut prop = commands.spawn((WorldAssetRoot(scene.clone()), transform));
    if let Some(constructor) = collider {
        prop.insert((
            RigidBody::Static,
            ColliderConstructorHierarchy::new(constructor),
        ));
    }
}
