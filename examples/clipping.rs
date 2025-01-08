use bevy::{
    color::palettes::css::{BLUE, GREEN, RED},
    prelude::*,
    render::primitives::HalfSpace,
};
use bevy_polyline::{clipping::ClippingSettings, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PolylinePlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, update_clipping)
        .run();
}

fn setup(
    mut commands: Commands,
    mut polyline_materials: ResMut<Assets<PolylineMaterial>>,
    mut polylines: ResMut<Assets<Polyline>>,
) {
    commands.spawn(PolylineBundle {
        polyline: polylines.add(Polyline {
            vertices: vec![
                // bottom face
                Vec3::new(-1.0, -1.0, -1.0),
                Vec3::new(1.0, -1.0, -1.0),
                Vec3::new(1.0, -1.0, 1.0),
                Vec3::new(-1.0, -1.0, 1.0),
                Vec3::new(-1.0, -1.0, -1.0),
                // vertical edges
                Vec3::new(-1.0, 1.0, -1.0),
                Vec3::new(-1.0, 1.0, 1.0),
                Vec3::new(-1.0, -1.0, 1.0),
                Vec3::new(-1.0, 1.0, 1.0),
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::new(1.0, -1.0, 1.0),
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::new(1.0, 1.0, -1.0),
                Vec3::new(1.0, -1.0, -1.0),
                Vec3::new(1.0, 1.0, -1.0),
                Vec3::new(-1.0, 1.0, -1.0),
            ],
        }),
        material: polyline_materials.add(PolylineMaterial {
            color: RED.into(),
            enable_clipping: true,
            ..default()
        }),
        ..default()
    });

    commands.spawn(PolylineBundle {
        polyline: polylines.add(Polyline {
            vertices: vec![
                // bottom face
                Vec3::new(-1.5, -1.5, -1.5),
                Vec3::new(1.5, -1.5, -1.5),
                Vec3::new(1.5, -1.5, 1.5),
                Vec3::new(-1.5, -1.5, 1.5),
                Vec3::new(-1.5, -1.5, -1.5),
                // vertical edges
                Vec3::new(-1.5, 1.5, -1.5),
                Vec3::new(-1.5, 1.5, 1.5),
                Vec3::new(-1.5, -1.5, 1.5),
                Vec3::new(-1.5, 1.5, 1.5),
                Vec3::new(1.5, 1.5, 1.5),
                Vec3::new(1.5, -1.5, 1.5),
                Vec3::new(1.5, 1.5, 1.5),
                Vec3::new(1.5, 1.5, -1.5),
                Vec3::new(1.5, -1.5, -1.5),
                Vec3::new(1.5, 1.5, -1.5),
                Vec3::new(-1.5, 1.5, -1.5),
            ],
        }),
        material: polyline_materials.add(PolylineMaterial {
            color: GREEN.into(),
            perspective: true,
            enable_clipping: true,
            ..default()
        }),
        ..default()
    });

    commands.spawn(PolylineBundle {
        polyline: polylines.add(Polyline {
            vertices: vec![Vec3::NEG_ONE, Vec3::ONE],
        }),
        material: polyline_materials.add(PolylineMaterial {
            color: BLUE.into(),
            enable_clipping: false,
            ..default()
        }),
        ..default()
    });

    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(2.5, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        camera: Camera {
            hdr: true,
            ..default()
        },
        ..default()
    });
}

fn update_clipping(mut settings: ResMut<ClippingSettings>, time: Res<Time>) {
    settings.clear();
    settings.push(HalfSpace::new(Vec4::new(
        1.0,
        0.0,
        1.0,
        time.elapsed_seconds().sin() + 2.0,
    )));
    settings.push(HalfSpace::new(Vec4::new(
        0.0,
        1.0,
        0.0,
        time.elapsed_seconds().cos() * 0.5 + 1.0,
    )));
}
