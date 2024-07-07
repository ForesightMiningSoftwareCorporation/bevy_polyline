use bevy::prelude::*;
use bevy_polyline::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PolylinePlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut polyline_materials: ResMut<Assets<PolylineMaterial>>,
    mut polylines: ResMut<Assets<Polyline>>,
) {
    commands.spawn(PolylineBundle {
        polyline: polylines.add(Polyline {
            vertices: vec![-Vec3::ONE, Vec3::ONE],
        }),
        material: polyline_materials.add(PolylineMaterial {
            width: 10.0,
            color: bevy::color::palettes::css::RED.into(),
            perspective: false,
            ..default()
        }),
        ..default()
    });

    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        camera: Camera {
            hdr: true,
            ..default()
        },
        ..default()
    });
}
