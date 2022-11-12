use bevy::prelude::*;
use bevy_polyline::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(PolylinePlugin)
        .add_startup_system(setup)
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
            ..Default::default()
        }),
        material: polyline_materials.add(PolylineMaterial {
            width: 10.0,
            color: Color::RED,
            perspective: false,
            ..Default::default()
        }),
        ..Default::default()
    });

    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        camera: Camera {
            hdr: true,
            ..default()
        },
        ..Camera3dBundle::default()
    });
}
