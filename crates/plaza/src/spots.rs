//! The portfolio spots: their props, the words around them (see `signs`),
//! their link pads, and a trigger zone for each. Entering a zone tells the
//! page, which reads that spot out to screen readers and, for spots with more
//! to say than the plaza shows, floats a card with the details over the thing
//! it is about.
//!
//! The plaza is 60 m square with the skater starting near the south edge,
//! facing north (−Z). The career line runs west to east, oldest to newest,
//! along the north edge. Ramps, rails, steps and the start box are Kenney
//! models; the rest is primitives. Words on the ground read from the south.

use std::f32::consts::{FRAC_PI_2, PI};

use avian3d::prelude::*;
use bevy::{light::NotShadowCaster, prelude::*, transform::TransformSystems};
use content::{LinkKind, Spot, Text};

use crate::{
    Layer,
    assets::{self, Models},
    bridge,
    loading::Phase,
    palette,
    signs::{self, Letters, LinkPad, PadLook, Words},
    skater::Skater,
    text_mesh::Style,
};

/// Where the skater starts, inside the start ledge's zone.
pub const START: Vec3 = Vec3::new(0.0, 0.0, 20.5);

const CAREER_Z: f32 = -24.0;
/// Room for the career line along X; the stops spread out to fill it.
const CAREER_LENGTH: f32 = 46.0;
const MAX_CAREER_SPACING: f32 = 3.4;
const BOARD_DEPTH: f32 = 0.12;

#[derive(Component)]
struct Zone(Spot);

/// The point a zone's details card points at.
#[derive(Component)]
struct Card(Vec3);

/// The zone the skater is in, if any.
#[derive(Resource, Default)]
struct InZone(Option<Entity>);

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

    /// Words painted on the ground, the top of their first line centred on
    /// `at` (x, z).
    fn paint(&mut self, text: Text, style: Style, at: Vec2, material: &Handle<StandardMaterial>) {
        self.commands.spawn((
            Words { text, style },
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(at.x, 0.02, at.y).with_rotation(flat()),
            NotShadowCaster,
        ));
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
    // and the profile links either side of the way north. Kenney's box has
    // sloped ends; turned to run east-west.
    let ledge = Transform::from_xyz(0.0, 0.0, 24.5)
        .with_rotation(Quat::from_rotation_y(FRAC_PI_2))
        .with_scale(Vec3::new(1.4, 2.2, 5.0));
    b.prop(|m| &m.obstacle_box, ledge, hull.clone());
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

    // Career line: a kicker per job or project, oldest in the west, with its
    // name and dates painted in front and each new year before that.
    let career = p.career();
    let stops = career.len();
    let spacing = (CAREER_LENGTH / stops as f32).min(MAX_CAREER_SPACING);
    let first = -spacing * (stops - 1) as f32 / 2.0;
    let path = Vec2::new(spacing * stops as f32 + 1.0, 5.0);
    b.commands.spawn((
        Mesh3d(b.meshes.add(Plane3d::new(Vec3::Y, path / 2.0))),
        MeshMaterial3d(concrete_dark.clone()),
        Transform::from_xyz(0.0, 0.005, CAREER_Z),
    ));
    b.paint(
        upper(&p.ui.experience),
        Style::new(0.8),
        Vec2::new(0.0, -16.6),
        &ink,
    );
    let mut year = None;
    for (i, stop) in career.iter().enumerate() {
        let x = first + i as f32 * spacing;
        let kicker = Transform::from_xyz(x, 0.0, CAREER_Z)
            .with_rotation(Quat::from_rotation_y(-FRAC_PI_2))
            .with_scale(Vec3::new(2.2, 1.3, 1.8));
        b.prop(|m| &m.ramp, kicker, hull.clone());
        let width = spacing - 0.4;
        b.paint(
            signs::each(|lang| stop.period(&p, lang)),
            Style::new(0.17).width(width).fit(),
            Vec2::new(x, -21.1),
            &metal,
        );
        let name = stop
            .project
            .map_or(&stop.job.company, |project| &project.name);
        b.paint(
            name.clone(),
            Style::new(0.22).width(width),
            Vec2::new(x, -20.7),
            &ink,
        );
        let started = stop.start().year;
        if year != Some(started) {
            year = Some(started);
            b.paint(
                Text::Same(started.to_string()),
                Style::new(0.5),
                Vec2::new(x, -18.8),
                &accent,
            );
        }
        // The zone takes in the words in front, and the card points above the kicker.
        let zone = b.zone(
            Spot::Career(i),
            Vec2::new(x, CAREER_Z + 1.4),
            Vec2::new(spacing, 7.2),
        );
        b.commands
            .entity(zone)
            .insert(Card(Vec3::new(x, 1.4, CAREER_Z)));
    }

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
    // the south end and one for its demo at the north end. Rails have no
    // colliders: the board locks onto one from above (see `Rails::catch`) and
    // rides through them on the ground rather than snagging.
    let rail_scale = Vec3::new(2.0, 2.0, 8.0);
    let rail_top = 0.25 * rail_scale.y;
    let rail_x = |i: usize| -25.0 + i as f32 * 2.5;
    let mut rails = Vec::new();
    for (i, repo) in p.open_source.iter().enumerate() {
        let at = Vec3::new(rail_x(i), 0.0, -12.0);
        b.prop(
            |m| &m.rail_low,
            Transform::from_translation(at).with_scale(rail_scale),
            None,
        );
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
    // wall, and the skills on a menu board outside. Its front faces south.
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
    let menu = Vec2::new(3.6, 3.0);
    let menu_board = b.board(menu, Vec3::new(-19.0, 0.0, 3.4), 0.2, &ink);
    let groups = signs::each(|lang| {
        let names: Vec<_> = p.skills.iter().map(|group| group.name.get(lang)).collect();
        names.join("\n")
    });
    b.print(
        menu_board,
        menu.extend(BOARD_DEPTH),
        groups,
        Style::new(0.15).width(menu.x - 0.4).height(menu.y - 0.5),
        0.25,
    );
    // The card with the whole stack points at the top of the menu board.
    let zone = b.zone(Spot::SkillShop, Vec2::new(-22.5, 8.0), Vec2::new(11.0, 9.0));
    b.commands
        .entity(zone)
        .insert(Card(Vec3::new(-19.0, 0.2 + menu.y, 3.4)));

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
/// the page about that one. Neighbouring zones touch, so the skater can be in two.
fn track_zones(
    mut started: MessageReader<CollisionStart>,
    mut ended: MessageReader<CollisionEnd>,
    zones: Query<&Zone>,
    skaters: Query<(), With<Skater>>,
    mut in_zone: ResMut<InZone>,
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

    in_zone.0 = inside.last().copied();
    let active = in_zone
        .0
        .and_then(|zone| zones.get(zone).ok())
        .map(|zone| zone.0);
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
