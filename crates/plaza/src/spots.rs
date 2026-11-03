//! The portfolio spots: their props, the words around them (see `signs`),
//! their link pads, and a trigger zone for each. Entering a zone tells the
//! page, which reads that spot out to screen readers and, for spots with more
//! to say than the plaza shows, floats a card with the details over the thing
//! it is about.
//!
//! The plaza is 60 m across and 70 m deep, with the skater starting near the
//! south edge, facing north (−Z). The career line runs round the pump track
//! at the north end (see `pump_track`). Rails and steps are Kenney models;
//! the rest is primitives. Words on the ground read from the south.

use std::f32::consts::{FRAC_PI_2, PI};

use avian3d::prelude::*;
use bevy::{
    asset::RenderAssetUsages, light::NotShadowCaster, mesh::PrimitiveTopology, prelude::*,
    transform::TransformSystems,
};
use content::{LinkKind, Spot, Text};

use crate::{
    Layer,
    assets::{self, Models},
    bridge,
    camera::Frame,
    loading::Phase,
    palette,
    pump_track::{self, PumpTrack, Surface},
    signs::{self, Letters, LinkPad, PadLook, Words},
    skater::Skater,
    skill_list,
    text_mesh::{Font, Style},
};

/// Where the skater starts, inside the start ledge's zone.
pub const START: Vec3 = Vec3::new(0.0, 0.0, 20.5);

const BOARD_DEPTH: f32 = 0.12;
/// A zone marked `Slow` only counts while the skater goes slower than this,
/// and keeps counting until they go faster than `FAST`.
const SLOW: f32 = 3.5;
const FAST: f32 = 5.0;
/// The skills' patch of ground: its north-west corner, clear of the boards
/// leaning on the shop and the bench by the edge, and the room it has to
/// the south, before the contact spot.
const YARD_WEST: f32 = -27.2;
const YARD_NORTH: f32 = 6.2;
const YARD_DEPTH: f32 = 8.4;
const YARD_MARGIN: f32 = 0.5;

#[derive(Component)]
struct Zone(Spot);

/// A zone the skater has to slow down in for it to count, so riding past
/// a row of them doesn't flick through their cards.
#[derive(Component)]
struct Slow;

/// The point a zone's details card points at.
#[derive(Component)]
struct Card(Vec3);

/// The zone the skater is in that counts at their speed, if any.
#[derive(Resource, Default)]
pub struct InZone(Option<Entity>);

impl InZone {
    pub fn get(&self) -> Option<Entity> {
        self.0
    }
}

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
    app.init_resource::<InZone>()
        .add_systems(OnEnter(Phase::Playing), spawn)
        .add_systems(Update, track_zones)
        .add_systems(PostUpdate, place_card.after(TransformSystems::Propagate));
}

/// Lays flat text on the ground, read from the south.
fn flat() -> Quat {
    Quat::from_rotation_x(-FRAC_PI_2)
}

/// Text in capitals.
fn upper(text: &Text) -> Text {
    signs::each(|lang| text.get(lang).to_uppercase())
}

/// The bottom and top corners of a `Builder::funbox` `size` big, each
/// anticlockwise from above.
fn funbox_corners(size: Vec3) -> ([Vec3; 4], [Vec3; 4]) {
    let round = |half: Vec2, y: f32| {
        [(1.0, 1.0), (1.0, -1.0), (-1.0, -1.0), (-1.0, 1.0)]
            .map(|(x, z)| Vec3::new(x * half.x, y, z * half.y))
    };
    (
        round(size.xz() / 2.0, 0.0),
        round(size.xz() / 2.0 - size.y, size.y),
    )
}

/// A funbox's top and sides, each face flat.
fn funbox_mesh(size: Vec3) -> Mesh {
    let (bottom, top) = funbox_corners(size);
    let sides = (0..4).map(|i| {
        let j = (i + 1) % 4;
        [bottom[i], bottom[j], top[j], top[i]]
    });
    let positions: Vec<Vec3> = std::iter::once(top)
        .chain(sides)
        .flat_map(|[a, b, c, d]| [a, b, c, a, c, d])
        .collect();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_computed_flat_normals()
}

/// Spawns props with shared materials.
struct Builder<'w, 's, 'a> {
    commands: Commands<'w, 's>,
    models: &'a Models,
    meshes: &'a mut Assets<Mesh>,
    pad: Handle<StandardMaterial>,
    paper: Handle<StandardMaterial>,
    metal: Handle<StandardMaterial>,
}

impl Builder<'_, '_, '_> {
    fn prop(
        &mut self,
        scene: fn(&Models) -> &Handle<WorldAsset>,
        transform: Transform,
        collider: Option<ColliderConstructor>,
    ) {
        assets::prop(&mut self.commands, scene(self.models), transform, collider);
    }

    /// A static box `size` wide, tall and deep, with its bottom centre at `at`.
    fn block(&mut self, size: Vec3, at: Vec3, material: &Handle<StandardMaterial>) -> Entity {
        self.commands
            .spawn((
                Mesh3d(self.meshes.add(Cuboid::from_size(size))),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(at + Vec3::Y * size.y / 2.0),
                RigidBody::Static,
                Collider::cuboid(size.x, size.y, size.z),
            ))
            .id()
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

    /// A box `size` long (x), tall and deep (z), its bottom centred on `at`,
    /// ramped at 45° on every side, with a bolt near each corner of its top.
    fn funbox(&mut self, at: Vec3, size: Vec3, material: &Handle<StandardMaterial>) {
        let (bottom, top) = funbox_corners(size);
        let bolt = self
            .meshes
            .add(Extrusion::new(RegularPolygon::new(0.12, 6), 0.04));
        let metal = self.metal.clone();
        self.commands
            .spawn((
                Mesh3d(self.meshes.add(funbox_mesh(size))),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(at),
                RigidBody::Static,
                Collider::convex_hull(&[bottom, top].concat()).expect("a funbox has volume"),
            ))
            .with_children(|funbox| {
                for corner in top {
                    let inward = Vec3::new(corner.x.signum(), 0.0, corner.z.signum()) * 0.6;
                    funbox.spawn((
                        Mesh3d(bolt.clone()),
                        MeshMaterial3d(metal.clone()),
                        Transform::from_translation(corner - inward)
                            .with_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
                    ));
                }
            });
    }

    /// A board `size` across on two posts, facing south, its bottom edge
    /// `lift` above the ground at `at`. Returns it to print on.
    fn board(
        &mut self,
        size: Vec2,
        at: Vec3,
        lift: f32,
        material: &Handle<StandardMaterial>,
    ) -> Entity {
        let metal = self.metal.clone();
        for side in [-1.0, 1.0] {
            let post = Vec3::new(side * (size.x / 2.0 - 0.4), 0.0, -BOARD_DEPTH);
            self.block(
                Vec3::new(0.12, lift + size.y / 2.0, 0.12),
                at + post,
                &metal,
            );
        }
        self.block(size.extend(BOARD_DEPTH), at + Vec3::Y * lift, material)
    }

    /// Words on the front of a `block` `size` big, `from_top` below its top.
    fn print(&mut self, block: Entity, size: Vec3, text: Text, style: Style, from_top: f32) {
        let words = self
            .commands
            .spawn((
                Words { text, style },
                MeshMaterial3d(self.paper.clone()),
                Transform::from_xyz(0.0, size.y / 2.0 - from_top, size.z / 2.0 + 0.005),
                NotShadowCaster,
            ))
            .id();
        self.commands.entity(block).add_child(words);
    }

    /// Words on a surface, placed and turned by `transform` (see `Words`).
    fn write(
        &mut self,
        text: Text,
        style: Style,
        transform: Transform,
        material: &Handle<StandardMaterial>,
    ) {
        self.commands.spawn((
            Words { text, style },
            MeshMaterial3d(material.clone()),
            transform,
            NotShadowCaster,
        ));
    }

    /// Words painted on the ground, the top of their first line centred on
    /// `at` (x, z).
    fn paint(&mut self, text: Text, style: Style, at: Vec2, material: &Handle<StandardMaterial>) {
        let transform = Transform::from_xyz(at.x, 0.02, at.y).with_rotation(flat());
        self.write(text, style, transform, material);
    }

    /// A purple pad `size` (x, z) across with its label on it. Rolling onto
    /// it offers to open `url`.
    fn link(&mut self, url: String, label: Text, at: Vec2, size: Vec2) {
        let style = Style::new(0.3).width(size.x - 0.5).fit().middle();
        let plate = self.meshes.add(Plane3d::new(Vec3::Y, size / 2.0));
        self.commands.spawn((
            LinkPad {
                url,
                label: label.clone(),
            },
            Mesh3d(plate),
            MeshMaterial3d(self.pad.clone()),
            Transform::from_xyz(at.x, 0.012, at.y),
            Sensor,
            Collider::cuboid(size.x, 2.0, size.y),
            CollisionEventsEnabled,
            CollisionLayers::new(Layer::Zone, Layer::Default),
            children![(
                Words { text: label, style },
                MeshMaterial3d(self.paper.clone()),
                Transform::from_xyz(0.0, 0.006, 0.0).with_rotation(flat()),
                NotShadowCaster,
            )],
        ));
    }

    /// A trigger zone `size` (x, z) across, centred on `center` (x, z).
    fn zone(&mut self, spot: Spot, center: Vec2, size: Vec2) -> Entity {
        self.commands
            .spawn((
                Zone(spot),
                Transform::from_xyz(center.x, 1.0, center.y),
                Sensor,
                Collider::cuboid(size.x, 2.0, size.y),
                CollisionEventsEnabled,
                // Only the skater sets zones off, and the ground probe looks past them.
                CollisionLayers::new(Layer::Zone, Layer::Default),
            ))
            .id()
    }
}

fn spawn(
    commands: Commands,
    models: Res<Models>,
    pad: Res<PadLook>,
    mut font: ResMut<Font>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let p = content::load().expect("content/portfolio.ron is checked by the content tests");

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
    let mut b = Builder {
        commands,
        models: &models,
        meshes: &mut meshes,
        pad: pad.idle.clone(),
        paper: paper.clone(),
        metal: metal.clone(),
    };
    let hull = Some(ColliderConstructor::ConvexHullFromMesh);
    let short_name = |name: &str| Text::Same(name.rsplit('/').next().unwrap_or(name).to_owned());

    // Start: the name in letters to knock over, the title painted under them,
    // and the profile links either side of the way north. Behind it, a
    // funbox far enough back to stay out of the first view.
    b.funbox(Vec3::new(0.0, 0.0, 25.5), Vec3::new(8.0, 0.6, 4.0), &accent);
    // Sized so the whole name fits a phone held upright (see `camera`).
    b.commands.spawn((
        Letters {
            text: upper(&p.identity.name),
            size: 1.0,
            depth: 0.4,
            material: accent.clone(),
        },
        Transform::from_xyz(0.0, 0.0, 12.0),
    ));
    b.paint(
        p.identity.title.clone(),
        Style::new(0.36).width(12.0),
        Vec2::new(0.0, 13.4),
        &ink,
    );
    b.paint(
        p.identity.location.clone(),
        Style::new(0.24),
        Vec2::new(0.0, 14.3),
        &metal,
    );
    // LeetCode has the stair set.
    let links: Vec<_> = p
        .identity
        .links
        .iter()
        .filter(|link| link.kind != LinkKind::LeetCode)
        .collect();
    let middle = (links.len() - 1) as f32 / 2.0;
    for (i, link) in links.into_iter().enumerate() {
        let k = i as f32 - middle;
        b.link(
            link.url.to_string(),
            link.label.clone(),
            Vec2::new(k * 3.9 + k.signum() * 1.0, 18.2),
            Vec2::new(3.4, 1.3),
        );
    }
    b.zone(Spot::Start, Vec2::new(START.x, 21.5), Vec2::new(8.0, 7.0));

    // Career line: the pump track, with a table per job or project, oldest
    // first. Each table has its dates and name on top, reading the way the
    // track runs, and each new year is painted beside it in the loop's middle.
    let career = p.career();
    let track = PumpTrack::new(career.len());
    let surface = track.mesh(palette::CONCRETE_DARK, palette::METAL);
    b.commands.spawn((
        Surface,
        Collider::trimesh_from_mesh_with_config(&surface, TrimeshFlags::FIX_INTERNAL_EDGES)
            .expect("the track has triangles"),
        Mesh3d(b.meshes.add(surface)),
        MeshMaterial3d(matte(Color::WHITE)),
        RigidBody::Static,
    ));
    b.paint(
        upper(&p.ui.experience),
        Style::new(0.8),
        Vec2::new(0.0, -17.7),
        &ink,
    );
    b.paint(
        p.plaza.pump_hint.clone(),
        Style::new(0.26).width(4.8),
        Vec2::new(0.0, -16.5),
        &metal,
    );
    let across = pump_track::WIDTH - 0.6;
    let top = track.table_top();
    let mut year = None;
    for (i, stop) in career.iter().enumerate() {
        let (table, direction) = track.table(i);
        let ahead = Vec3::new(direction.x, 0.0, direction.y);
        let reading = Quat::from_rotation_arc(Vec3::NEG_Z, ahead) * flat();
        let on_top = |along: f32| {
            Transform::from_translation(table + ahead * along + Vec3::Y * 0.01)
                .with_rotation(reading)
        };
        b.write(
            signs::each(|lang| stop.period(&p, lang)),
            Style::new(0.16).width(across).fit(),
            on_top(top / 2.0 - 0.1),
            &metal,
        );
        let name = stop
            .project
            .map_or(&stop.job.company, |project| &project.name);
        b.write(
            name.clone(),
            Style::new(0.28).width(across).height(top - 0.6),
            on_top(top / 2.0 - 0.45),
            &ink,
        );
        let started = stop.start().year;
        if year != Some(started) {
            year = Some(started);
            let beside = track.beside(i);
            b.write(
                Text::Same(started.to_string()),
                Style::new(0.6).middle(),
                Transform::from_xyz(beside.x, 0.02, beside.y).with_rotation(reading),
                &accent,
            );
        }
        // A rider going past at speed only reads the table; slowing down on
        // it brings up the card, which points above it.
        let zone = b.zone(
            Spot::Career(i),
            table.xz(),
            Vec2::new(track.pitch(), track.full_width()),
        );
        b.commands
            .entity(zone)
            .insert((Slow, Card(table + Vec3::Y)));
    }
    b.commands.insert_resource(track);

    // Big gap: a kicker, a three-metre gap, a platform to land on, a ramp
    // down, and the highlights on boards along the far side.
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
    b.paint(
        upper(&p.ui.highlights),
        Style::new(0.7),
        Vec2::new(15.5, 8.4),
        &ink,
    );
    let highlight_spacing = (16.0 / p.highlights.len() as f32).min(4.0);
    for (i, highlight) in p.highlights.iter().enumerate() {
        let size = Vec2::new(highlight_spacing - 0.4, 1.4);
        let x = 9.5 + i as f32 * highlight_spacing;
        let board = b.board(size, Vec3::new(x, 0.0, -0.4), 0.9, &ink);
        b.print(
            board,
            size.extend(BOARD_DEPTH),
            highlight.title.clone(),
            Style::new(0.2).width(size.x - 0.4).height(size.y - 0.4),
            0.25,
        );
    }
    b.zone(Spot::Gap, Vec2::new(15.5, gap_z), Vec2::new(16.0, 6.0));

    // Rail garden: a rail per open-source repo, with a pad for its code at
    // the south end and one for its demo at the north end. The board locks
    // onto a rail from above (see `Rails::catch`); from the side, a wall under
    // the bar stops it. The wall stays just below the bar's top, so a board
    // grinding along the top doesn't scrape it.
    let rail_scale = Vec3::new(2.0, 2.0, 8.0);
    let rail_top = 0.25 * rail_scale.y;
    let rail_x = |i: usize| -25.0 + i as f32 * 2.5;
    let mut rails = Vec::new();
    for (i, repo) in p.open_source.iter().enumerate() {
        let at = Vec3::new(rail_x(i), 0.0, -12.0);
        let wall = Vec3::new(0.11 * rail_scale.x, rail_top - 0.05, rail_scale.z);
        b.prop(
            |m| &m.rail_low,
            Transform::from_translation(at).with_scale(rail_scale),
            None,
        );
        b.commands.spawn((
            Transform::from_translation(at + Vec3::Y * wall.y / 2.0),
            RigidBody::Static,
            Collider::cuboid(wall.x, wall.y, wall.z),
        ));
        let half = Vec3::Z * rail_scale.z / 2.0;
        rails.push(Rail {
            a: at - half + Vec3::Y * rail_top,
            b: at + half + Vec3::Y * rail_top,
        });
        let pad = Vec2::new(2.3, 1.1);
        b.link(
            repo.url.to_string(),
            short_name(&repo.name),
            Vec2::new(at.x, -6.7),
            pad,
        );
        if let Some(demo) = &repo.demo {
            b.link(
                demo.to_string(),
                p.ui.demo.clone(),
                Vec2::new(at.x, -17.3),
                pad,
            );
        }
        b.zone(Spot::Rail(i), at.xz(), Vec2::new(1.8, 9.0));
    }
    b.commands.insert_resource(Rails(rails));
    let garden = (rail_x(0) + rail_x(p.open_source.len().saturating_sub(1))) / 2.0;
    b.paint(
        upper(&p.ui.open_source),
        Style::new(0.6),
        Vec2::new(garden, -4.6),
        &ink,
    );

    // Skill shop: a skate shop with a sign on the roof, a striped awning, a
    // window with boards on display, a door, a giant board leaning on the
    // wall, and the skills on the ground outside. Its front faces south.
    let front = 4.75;
    b.block(Vec3::new(5.0, 3.0, 3.5), Vec3::new(-24.5, 0.0, 3.0), &paper);
    let sign = Vec3::new(3.4, 0.8, 0.15);
    let roof_sign = b.block(sign, Vec3::new(-24.5, 3.0, front - 0.3), &ink);
    b.print(
        roof_sign,
        sign,
        upper(&p.ui.skills),
        Style::new(0.42).middle(),
        sign.y / 2.0,
    );
    for stripe in 0..6 {
        let x = -27.2 + 0.45 + stripe as f32 * 0.9;
        let colour = if stripe % 2 == 0 { &accent } else { &paper };
        b.block(
            Vec3::new(0.9, 0.15, 1.2),
            Vec3::new(x, 2.6, front + 0.55),
            colour,
        );
    }
    b.block(
        Vec3::new(2.2, 1.5, 0.04),
        Vec3::new(-24.6, 0.8, front + 0.02),
        &metal,
    );
    b.block(
        Vec3::new(2.0, 1.3, 0.06),
        Vec3::new(-24.6, 0.9, front + 0.03),
        &ink,
    );
    b.block(
        Vec3::new(0.9, 2.0, 0.06),
        Vec3::new(-22.8, 0.0, front + 0.03),
        &accent,
    );
    b.block(
        Vec3::new(0.08, 0.08, 0.1),
        Vec3::new(-22.5, 1.0, front + 0.08),
        &metal,
    );
    // A board leaning on the wall, graphic and wheels facing out: stood on its
    // tail with the deck's top against the wall, tipped 12° back.
    let mut leaning_board = |x: f32, scale: f32, collider: Option<ColliderConstructor>| {
        let (length, thickness) = (0.69 * scale, 0.18 * scale);
        let lean = 12f32.to_radians();
        let at = Vec3::new(
            x,
            length / 2.0 * lean.cos(),
            front + 0.02 + thickness + length / 2.0 * lean.sin(),
        );
        let transform = Transform::from_translation(at)
            .with_rotation(Quat::from_rotation_x(-(FRAC_PI_2 + lean)))
            .with_scale(Vec3::splat(scale));
        b.prop(|m| &m.skateboard, transform, collider);
    };
    leaning_board(-26.3, 4.0, hull.clone());
    for x in [-25.3, -24.6, -23.9] {
        leaning_board(x, 1.6, None);
    }
    // The skills, on grip tape on the ground in front of the shop and read
    // from the south (see `skill_list`). Slowing down on them brings the
    // camera round to look straight down at them, close enough to read.
    let list = skill_list::lay_out(&mut font, &p.skills, YARD_DEPTH - 2.0 * YARD_MARGIN);
    let grip = Vec2::new(list.width, list.height) + 2.0 * YARD_MARGIN;
    let yard = Vec2::new(YARD_WEST + grip.x / 2.0, YARD_NORTH + grip.y / 2.0);
    b.commands.spawn((
        Mesh3d(b.meshes.add(Plane3d::new(Vec3::Y, grip / 2.0))),
        MeshMaterial3d(ink.clone()),
        Transform::from_xyz(yard.x, 0.01, yard.y),
    ));
    let lilac = matte(palette::ACCENT_LIGHT);
    for block in &list.blocks {
        let top = Vec2::new(YARD_WEST, YARD_NORTH) + YARD_MARGIN + Vec2::from(block.top);
        b.paint(
            block.heading.clone(),
            skill_list::heading_style(),
            top,
            &lilac,
        );
        let list_top = top + Vec2::Y * block.list_below;
        b.paint(
            block.list.clone(),
            skill_list::list_style(),
            list_top,
            &paper,
        );
    }
    let south = YARD_NORTH + grip.y;
    let zone = b.zone(
        Spot::SkillShop,
        Vec2::new(yard.x, (front + south) / 2.0),
        Vec2::new(grip.x, south - front),
    );
    b.commands.entity(zone).insert((
        Slow,
        Frame {
            centre: Vec3::new(yard.x, 0.0, yard.y),
            across: Dir3::X,
            up: Dir3::NEG_Z,
            toward: Dir3::Y,
            size: grip,
            view: Vec2::new(skill_list::COLUMN + 0.8, 5.0),
        },
    ));

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
    b.paint(
        upper(&p.ui.contact),
        Style::new(0.6),
        Vec2::new(-20.8, 15.6),
        &ink,
    );
    b.paint(
        Text::Same(p.identity.email.clone()),
        Style::new(0.3),
        Vec2::new(-20.8, 16.8),
        &ink,
    );
    b.link(
        format!("mailto:{}", p.identity.email),
        p.ui.email_me.clone(),
        Vec2::new(-21.9, 19.4),
        Vec2::new(2.6, 1.2),
    );
    b.link(
        format!("/{}", p.site.cv),
        p.ui.download_cv.clone(),
        Vec2::new(-18.6, 19.4),
        Vec2::new(3.4, 1.2),
    );
    b.zone(
        Spot::PhoneBooth,
        Vec2::new(-21.0, 18.5),
        Vec2::new(8.0, 7.0),
    );

    // Billboard: the app on the screen. It faces the start, and the board
    // sits above the trailing camera, so riding between the posts doesn't
    // blank the view.
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
    let screen_size = Vec3::new(5.4, 2.4, 0.05);
    let screen = b.block(
        screen_size,
        Vec3::new(0.0, BOARD_BOTTOM + 0.3, -13.83),
        &accent,
    );
    let app = &p.apps[0];
    b.print(
        screen,
        screen_size,
        Text::Same(app.name.clone()),
        Style::new(0.65),
        0.35,
    );
    b.print(
        screen,
        screen_size,
        app.tagline.clone(),
        Style::new(0.22).width(screen_size.x - 0.6).fit(),
        1.4,
    );
    for (x, url, label) in [
        (-2.4, &app.web, &p.ui.open_web),
        (2.4, &app.studio, &p.ui.studio_page),
    ] {
        b.link(
            url.to_string(),
            label.clone(),
            Vec2::new(x, -8.4),
            Vec2::new(3.0, 1.2),
        );
    }
    b.zone(Spot::Billboard, Vec2::new(0.0, -9.0), Vec2::new(8.0, 6.0));

    // Stair set: Kenney's three steps, rising toward −Z, with LeetCode and
    // the repo of solutions in front.
    let steps = Transform::from_xyz(20.0, 0.0, -12.25).with_scale(Vec3::new(5.0, 3.0, 4.5));
    b.prop(
        |m| &m.steps,
        steps,
        Some(ColliderConstructor::TrimeshFromMesh),
    );
    b.paint(
        p.plaza.stairs_title.clone(),
        Style::new(0.42),
        Vec2::new(20.0, -5.4),
        &ink,
    );
    if let Some(leetcode) = p.identity.link(LinkKind::LeetCode) {
        b.link(
            leetcode.url.to_string(),
            leetcode.label.clone(),
            Vec2::new(17.9, -7.8),
            Vec2::new(3.4, 1.2),
        );
    }
    if let Some(repo) = p.stairs_repo() {
        b.link(
            repo.url.to_string(),
            short_name(&repo.name),
            Vec2::new(22.1, -7.8),
            Vec2::new(3.4, 1.2),
        );
    }
    b.zone(Spot::Stairs, Vec2::new(20.0, -8.0), Vec2::new(7.0, 5.0));

    // The Bibacode sticker, hidden in the south-east corner. Finding it
    // offers the Telegram channel.
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
    b.paint(
        p.plaza.egg_title.clone(),
        Style::new(0.2),
        Vec2::new(26.0, 25.8),
        &metal,
    );
    b.paint(
        p.plaza.egg_body.clone(),
        Style::new(0.14),
        Vec2::new(26.0, 26.3),
        &metal,
    );
    if let Some(telegram) = p.identity.link(LinkKind::Telegram) {
        b.commands.spawn((
            LinkPad {
                url: telegram.url.to_string(),
                label: telegram.label.clone(),
            },
            Transform::from_translation(sticker),
            Sensor,
            Collider::cuboid(3.0, 2.0, 3.0),
            CollisionEventsEnabled,
            CollisionLayers::new(Layer::Zone, Layer::Default),
        ));
    }
    b.zone(Spot::Sticker, sticker.xz(), Vec2::splat(3.0));
}

/// Keeps the zones the skater is in, most recently entered last, and tells
/// the page about the latest one that counts at the skater's speed.
/// Neighbouring zones touch, so the skater can be in two.
#[allow(clippy::too_many_arguments)]
fn track_zones(
    mut started: MessageReader<CollisionStart>,
    mut ended: MessageReader<CollisionEnd>,
    zones: Query<(&Zone, Has<Slow>)>,
    skaters: Query<&LinearVelocity, With<Skater>>,
    mut in_zone: ResMut<InZone>,
    mut inside: Local<Vec<Entity>>,
    mut slow: Local<bool>,
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

    let speed = skaters.single().map_or(0.0, |velocity| velocity.length());
    *slow = speed < SLOW || (*slow && speed < FAST);
    in_zone.0 = inside.iter().rev().copied().find(|&zone| {
        zones
            .get(zone)
            .is_ok_and(|(_, needs_slow)| *slow || !needs_slow)
    });
    let active = in_zone
        .0
        .and_then(|zone| zones.get(zone).ok())
        .map(|(zone, _)| zone.0);
    if active != *shown {
        *shown = active;
        match active {
            Some(spot) => bridge::show(spot),
            None => bridge::hide(),
        }
    }
}

/// Keeps the card of the zone the skater is in pointing at its spot as the
/// camera moves, in the page's pixels.
fn place_card(
    in_zone: Res<InZone>,
    cards: Query<&Card>,
    camera: Single<(&Camera, &GlobalTransform)>,
    mut placed: Local<Option<Vec2>>,
) {
    let (camera, view) = *camera;
    let at = in_zone
        .0
        .and_then(|zone| cards.get(zone).ok())
        .and_then(|card| camera.world_to_viewport(view, card.0).ok())
        .map(Vec2::round);
    if at != *placed {
        *placed = at;
        bridge::card(at);
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_4;

    use super::*;

    /// The skills fit their patch of ground, clear of the way north from the
    /// start.
    #[test]
    fn skills_fit_the_yard() {
        let portfolio = content::load().unwrap();
        let depth = YARD_DEPTH - 2.0 * YARD_MARGIN;
        let list = skill_list::lay_out(&mut Font::default(), &portfolio.skills, depth);
        assert!(list.height <= depth, "{} m deep", list.height);
        let east = YARD_WEST + list.width + 2.0 * YARD_MARGIN;
        assert!(east < -8.0, "reaches x = {east}");
    }

    /// Every face of a funbox faces out, and every side rises at 45°.
    #[test]
    fn funbox_ramps_at_45() {
        let size = Vec3::new(8.0, 0.6, 4.0);
        let mesh = funbox_mesh(size);
        let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
        let normals = mesh.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap();
        let (Some(positions), Some(normals)) = (positions.as_float3(), normals.as_float3()) else {
            panic!("positions and normals are 3D");
        };
        let middle = Vec3::Y * size.y / 2.0;
        for (triangle, normals) in positions.chunks(3).zip(normals.chunks(3)) {
            let centre = triangle.iter().map(|&p| Vec3::from(p)).sum::<Vec3>() / 3.0;
            let normal = Vec3::from(normals[0]);
            assert!(normal.dot(centre - middle) > 0.0, "{normal} faces in");
            if normal.y < 0.99 {
                assert!(
                    (normal.y - FRAC_PI_4.cos()).abs() < 1e-5,
                    "{normal} isn't 45°"
                );
            }
        }
    }
}
