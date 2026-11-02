//! Renders the social preview image: `cargo run -p plaza -- --og-image <path>`
//! frames the plaza from behind the start, renders it into a 1200×630 image
//! (off-screen, so it doesn't matter whether the window is visible), saves it
//! to `<path>` and exits. Native only; the README has the follow-up steps.
//!
//! `--camera x,y,z,tx,ty,tz` shoots from another point toward another target,
//! which is handy for checking a part of the plaza without playing to it.

use std::path::PathBuf;

use bevy::{
    camera::RenderTarget,
    prelude::*,
    render::{
        render_resource::{TextureFormat, TextureUsages},
        view::screenshot::{Screenshot, save_to_disk},
    },
    window::WindowResolution,
};

use crate::{camera, loading::Phase};

pub const SIZE: (u32, u32) = (1200, 630);
/// Frames to wait after the plaza is built, so shaders and shadows are ready.
const SETTLE_FRAMES: u32 = 90;

#[derive(Resource)]
struct Shot {
    path: PathBuf,
    image: Handle<Image>,
    camera: Transform,
    frames: u32,
}

/// Behind and above the start, looking past the skater at the name.
fn preview_camera() -> Transform {
    Transform::from_xyz(-2.0, 3.6, 26.5).looking_at(Vec3::new(2.5, 0.8, 13.0), Vec3::Y)
}

/// The pose after `--camera`, as `x,y,z,tx,ty,tz`.
fn requested_camera() -> Option<Transform> {
    let mut args = std::env::args().skip_while(|arg| arg != "--camera");
    args.next()?;
    let numbers: Vec<f32> = args
        .next()?
        .split(',')
        .map(|n| n.parse().ok())
        .collect::<Option<_>>()?;
    let [x, y, z, tx, ty, tz] = numbers[..] else {
        return None;
    };
    Some(Transform::from_xyz(x, y, z).looking_at(Vec3::new(tx, ty, tz), Vec3::Y))
}

/// The path after `--og-image`, if the app was started to render the preview.
pub fn requested() -> Option<PathBuf> {
    let mut args = std::env::args().skip_while(|arg| arg != "--og-image");
    args.next()?;
    args.next().map(PathBuf::from)
}

pub fn window_resolution() -> WindowResolution {
    WindowResolution::new(SIZE.0, SIZE.1).with_scale_factor_override(1.0)
}

pub fn plugin(path: PathBuf) -> impl Fn(&mut App) {
    move |app: &mut App| {
        let mut image =
            Image::new_target_texture(SIZE.0, SIZE.1, TextureFormat::Bgra8UnormSrgb, None);
        // The screenshot copies the texture back from the GPU.
        image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
        let image = app.world_mut().resource_mut::<Assets<Image>>().add(image);
        app.insert_resource(Shot {
            path: path.clone(),
            image,
            camera: requested_camera().unwrap_or_else(preview_camera),
            frames: 0,
        })
        .add_systems(OnEnter(Phase::Playing), render_to_image)
        .add_systems(
            PostUpdate,
            frame_and_shoot
                .after(camera::follow)
                .before(TransformSystems::Propagate)
                .run_if(in_state(Phase::Playing)),
        );
    }
}

fn render_to_image(
    mut commands: Commands,
    shot: Res<Shot>,
    camera: Single<Entity, With<Camera3d>>,
) {
    commands
        .entity(*camera)
        .insert(RenderTarget::Image(shot.image.clone().into()));
}

fn frame_and_shoot(
    mut commands: Commands,
    mut shot: ResMut<Shot>,
    mut camera: Single<&mut Transform, With<Camera3d>>,
    mut exit: MessageWriter<AppExit>,
) {
    **camera = shot.camera;
    shot.frames += 1;
    if shot.frames == SETTLE_FRAMES {
        commands
            .spawn(Screenshot::image(shot.image.clone()))
            .observe(save_to_disk(shot.path.clone()));
    }
    if shot.frames == SETTLE_FRAMES + 30 {
        exit.write(AppExit::Success);
    }
}
