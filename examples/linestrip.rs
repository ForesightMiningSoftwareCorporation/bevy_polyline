use bevy::{pbr::PointLightBundle, prelude::*};
use bevy_polyline::prelude::*;

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_plugin(PolylinePlugin)
        .add_startup_system(setup)
        .add_system(rotator_system)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut polyline_materials: ResMut<Assets<PolylineMaterial>>,
    mut polylines: ResMut<Assets<Polyline>>,
) {
    commands.spawn(PolylineBundle {
        polyline: polylines.add(Polyline {
            vertices: vec![
                Vec3::new(-0.5, -0.5, -0.5),
                Vec3::new(0.5, -0.5, -0.5),
                Vec3::new(0.5, 0.5, -0.5),
                Vec3::new(-0.5, 0.5, -0.5),
                Vec3::new(-0.5, 0.5, 0.5),
                Vec3::new(0.5, 0.5, 0.5),
                Vec3::new(0.5, -0.5, 0.5),
                Vec3::new(-0.5, -0.5, 0.5),
            ],
            ..Default::default()
        }),
        material: polyline_materials.add(PolylineMaterial {
            width: 2.0,
            color: Color::RED,
            perspective: false,
            // Bias the line toward the camera so the line at the cube-plane intersection is visible
            depth_bias: -0.0002,
            ..Default::default()
        }),
        ..Default::default()
    });

    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
        material: standard_materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        transform: Transform::from_xyz(0.0, -0.5, 0.0),
        ..Default::default()
    });

    // cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: standard_materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
        ..Default::default()
    });

    // light
    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..Default::default()
    });

    // camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(-2.0, 2.5, -5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..Camera3dBundle::default()
        },
        Rotates,
    ));
}

/// this component indicates what entities should rotate
#[derive(Component)]
struct Rotates;

fn rotator_system(time: Res<Time>, mut query: Query<&mut Transform, With<Rotates>>) {
    for mut transform in query.iter_mut() {
        *transform = Transform::from_rotation(Quat::from_rotation_y(
            (4.0 * std::f32::consts::PI / 20.0) * time.delta_seconds(),
        )) * *transform;
    }
}
