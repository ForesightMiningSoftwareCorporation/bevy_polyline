use std::f32::consts::TAU;
use std::f64::consts::TAU as TAU64;

use bevy::prelude::*;
use bevy_polyline::prelude::*;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_plugin(PolylinePlugin)
        .add_system(rotate_plane)
        .add_startup_system(setup)
        .run();
}

#[derive(Component)]
struct Rotating(f64);

fn rotate_plane(time: Res<Time>, mut animated: Query<(&mut Transform, &Rotating)>) {
    let time = time.seconds_since_startup();
    for (mut trans, Rotating(period)) in animated.iter_mut() {
        let angle = time % period / period * TAU64;
        let rot = Quat::from_rotation_y(angle as f32);
        trans.rotation = Quat::from_rotation_y(TAU / 4.1) * rot;
    }
}
fn setup(
    mut cmds: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut pbr_materials: ResMut<Assets<StandardMaterial>>,
    mut polylines: ResMut<Assets<Polyline>>,
    mut materials: ResMut<Assets<PolylineMaterial>>,
) {
    cmds.spawn_bundle(PerspectiveCameraBundle::new_3d())
        .insert(Transform::from_xyz(100.0, 0.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y));
    cmds.spawn_bundle(PbrBundle {
        mesh: meshes.add(shape::Box::new(0.01, 100.0, 10000.0).into()),
        material: pbr_materials.add(Color::WHITE.into()),
        ..default()
    })
    .insert(Rotating(30.0));
    let top = Vec3::Y * 100.0;
    let bottom = Vec3::Y * -100.0;
    // Show the middle point as a vertical red bar.
    cmds.spawn_bundle(PolylineBundle {
        polyline: polylines.add(Polyline {
            vertices: vec![top, bottom],
        }),
        material: materials.add(PolylineMaterial {
            width: 5.0,
            color: Color::RED,
            depth_bias: -1.0,
            perspective: false,
            ..Default::default()
        }),
        ..Default::default()
    });
    // Evaluate how "deep" the depth_bias makes the line change
    for i in 0..100 {
        let bias = (i as f32) / 50.0 - 1.0;
        let left = Vec3::new(0.0, bias * 35.0, -500.0);
        let right = Vec3::new(0.0, bias * 35.0, 500.0);
        cmds.spawn_bundle(PolylineBundle {
            polyline: polylines.add(Polyline {
                vertices: vec![left, right],
            }),
            material: materials.add(PolylineMaterial {
                width: 1.0,
                color: Color::hsl((bias + 1.0) / 2.0 * 270.0, 1.0, 0.5),
                depth_bias: bias,
                perspective: false,
                ..Default::default()
            }),
            ..Default::default()
        });
    }
}
