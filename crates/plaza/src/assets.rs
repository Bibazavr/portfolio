//! Models from Kenney's "Mini Skate" pack (CC0, see assets/models/LICENSE-kenney-mini-skate.txt),
//! with the shared palette texture recoloured to the studio's purple. They are
//! embedded in the binary: a few hundred kilobytes that load with the WASM,
//! need no extra requests and can't go stale next to it.

use std::path::{Path, PathBuf};

use avian3d::prelude::*;
use bevy::{asset::io::embedded::EmbeddedAssetRegistry, prelude::*};

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

/// The path of an embedded model, by file name without `.glb`.
pub fn model(name: &str) -> String {
    format!("embedded://models/{name}.glb")
}

/// A model's scene, ready to spawn with `WorldAssetRoot`.
pub fn scene(assets: &AssetServer, name: &str) -> Handle<WorldAsset> {
    assets.load(GltfAssetLabel::Scene(0).from_asset(model(name)))
}

/// Spawns a model that doesn't move. With a `collider`, colliders are built
/// from its meshes once it loads: a convex hull for simple solid shapes, a
/// triangle mesh for curved or stepped ones.
pub fn prop(
    commands: &mut Commands,
    assets: &AssetServer,
    name: &str,
    transform: Transform,
    collider: Option<ColliderConstructor>,
) {
    let mut prop = commands.spawn((WorldAssetRoot(scene(assets, name)), transform));
    if let Some(constructor) = collider {
        prop.insert((
            RigidBody::Static,
            ColliderConstructorHierarchy::new(constructor),
        ));
    }
}
