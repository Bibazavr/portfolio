//! Words in the plaza, in the visitor's language: painted on the ground,
//! printed on boards, spelled in solid letters the skater can knock over, and
//! on the link pads that offer to open a page. `spots` places them; this module
//! writes them, and writes them again when the visitor switches language.

use avian3d::prelude::*;
use bevy::prelude::*;
use content::{Lang, Text};

use crate::{
    bridge, palette,
    skater::Skater,
    text_mesh::{Font, Style},
};

/// Below this, a knocked-over letter has left the plaza.
const FELL_OFF: f32 = -5.0;

/// The language the plaza is written in.
#[derive(Resource, PartialEq)]
pub struct WorldLang(pub Lang);

/// Words written as a flat mesh on this entity, facing its +Z.
#[derive(Component)]
pub struct Words {
    pub text: Text,
    pub style: Style,
}

/// Words spelled in solid letters standing on the ground, centred on this
/// entity and facing its +Z.
#[derive(Component)]
pub struct Letters {
    pub text: Text,
    /// The height of a capital.
    pub size: f32,
    pub depth: f32,
    pub material: Handle<StandardMaterial>,
}

/// One of the solid letters, and where it stood.
#[derive(Component)]
struct Letter(Transform);

/// A sensor that offers to open `url` while the skater is on it. With a
/// mesh, it is a plate that lights up then.
#[derive(Component)]
pub struct LinkPad {
    pub url: String,
    pub label: Text,
}

/// The link pads' plates, idle and with the skater on them.
#[derive(Resource)]
pub struct PadLook {
    pub idle: Handle<StandardMaterial>,
    lit: Handle<StandardMaterial>,
}

impl FromWorld for PadLook {
    fn from_world(world: &mut World) -> Self {
        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        let mut plate = |color: Color| {
            materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.9,
                ..default()
            })
        };
        Self {
            idle: plate(palette::ACCENT),
            lit: plate(palette::ACCENT_LIGHT),
        }
    }
}

/// Text made the same way in each language.
pub fn each(write: impl Fn(Lang) -> String) -> Text {
    Text::Translated {
        en: write(Lang::En),
        ru: write(Lang::Ru),
    }
}

pub fn plugin(app: &mut App) {
    app.insert_resource(WorldLang(bridge::lang()))
        .init_resource::<Font>()
        .init_resource::<PadLook>()
        .add_systems(
            Update,
            (follow_lang, (write_words, stand_letters, track_pads)).chain(),
        )
        .add_systems(FixedUpdate, return_fallen);
}

fn follow_lang(mut lang: ResMut<WorldLang>) {
    if let Some(switched) = bridge::take_lang() {
        lang.set_if_neq(WorldLang(switched));
    }
}

fn write_words(
    lang: Res<WorldLang>,
    mut font: ResMut<Font>,
    words: Query<(Entity, Ref<Words>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut commands: Commands,
) {
    for (entity, words) in &words {
        if lang.is_changed() || words.is_added() {
            let mesh = font.flat(words.text.get(lang.0), words.style);
            commands.entity(entity).insert(Mesh3d(meshes.add(mesh)));
        }
    }
}

/// Stands the letters up, and again in the new language after a switch,
/// which also puts back any that were knocked over.
fn stand_letters(
    lang: Res<WorldLang>,
    mut font: ResMut<Font>,
    words: Query<(Ref<Letters>, &Transform)>,
    letters: Query<Entity, With<Letter>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut commands: Commands,
) {
    if !lang.is_changed() && !words.iter().any(|(words, _)| words.is_added()) {
        return;
    }
    for letter in &letters {
        commands.entity(letter).despawn();
    }
    for (words, origin) in &words {
        let gap = words.size * 0.12;
        let space = words.size * 0.5;
        let glyphs: Vec<_> = words
            .text
            .get(lang.0)
            .chars()
            .map(|letter| font.letter(letter, words.size, words.depth))
            .collect();
        let width = glyphs
            .iter()
            .map(|glyph| glyph.as_ref().map_or(space, |(_, bounds)| bounds.x) + gap)
            .sum::<f32>()
            - gap;
        let mut x = -width / 2.0;
        for glyph in glyphs {
            let Some((mesh, bounds)) = glyph else {
                x += space + gap;
                continue;
            };
            let home =
                origin.mul_transform(Transform::from_xyz(x + bounds.x / 2.0, bounds.y / 2.0, 0.0));
            commands.spawn((
                Letter(home),
                home,
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(words.material.clone()),
                RigidBody::Dynamic,
                Collider::cuboid(bounds.x, bounds.y, bounds.z),
                TransformInterpolation,
            ));
            x += bounds.x + gap;
        }
    }
}

/// A letter knocked over the plaza's edge comes back to where it stood.
fn return_fallen(
    mut letters: Query<(
        &Letter,
        &mut Position,
        &mut Rotation,
        &mut LinearVelocity,
        &mut AngularVelocity,
    )>,
) {
    for (Letter(home), mut position, mut rotation, mut linear, mut angular) in &mut letters {
        if position.y < FELL_OFF {
            position.0 = home.translation;
            rotation.0 = home.rotation;
            linear.0 = Vec3::ZERO;
            angular.0 = Vec3::ZERO;
        }
    }
}

/// Keeps the pads the skater is on, most recently entered last, lights that
/// one and offers its link.
#[allow(clippy::too_many_arguments)]
fn track_pads(
    mut started: MessageReader<CollisionStart>,
    mut ended: MessageReader<CollisionEnd>,
    mut pads: Query<(&LinkPad, Option<&mut MeshMaterial3d<StandardMaterial>>)>,
    skaters: Query<(), With<Skater>>,
    lang: Res<WorldLang>,
    look: Res<PadLook>,
    mut inside: Local<Vec<Entity>>,
    mut offered: Local<Option<Entity>>,
) {
    let entered = started.read().map(|e| (e.collider1, e.collider2, true));
    let left = ended.read().map(|e| (e.collider1, e.collider2, false));
    for (a, b, entered) in entered.chain(left) {
        let pad = if pads.contains(a) && skaters.contains(b) {
            a
        } else if pads.contains(b) && skaters.contains(a) {
            b
        } else {
            continue;
        };
        inside.retain(|&p| p != pad);
        if entered {
            inside.push(pad);
        }
    }
    inside.retain(|&pad| pads.contains(pad));

    let active = inside.last().copied();
    if active == *offered && !lang.is_changed() {
        return;
    }
    for (pad, lit) in [(*offered, false), (active, true)] {
        if let Some(Ok((_, Some(mut plate)))) = pad.map(|pad| pads.get_mut(pad)) {
            plate.0 = if lit { &look.lit } else { &look.idle }.clone();
        }
    }
    *offered = active;
    match active.and_then(|pad| pads.get(pad).ok()) {
        Some((pad, _)) => bridge::link(&pad.url, pad.label.get(lang.0)),
        None => bridge::unlink(),
    }
}
