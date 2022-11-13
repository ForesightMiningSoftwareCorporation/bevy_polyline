use std::f32::consts::TAU;
use std::f64::consts::TAU as TAU64;

use bevy::prelude::*;
use bevy_polyline::prelude::*;

// This example demonstrates how to use the `depth_bias` field on `PolylineMaterial`
//
// It should display on screen:
// * A rotating plane centered on the screen that eventually intersects with the camera.
// * A vertical red line that is drawn in front of everything.
// * 100 horizontal lines, going from the top to the bottom of the screen going through
//   all the colors of the rainbow.
//
// In addition, you can use the UP and DOWN arrow keys to move forward and backward the
// camera.
//
// Each horizontal line has a different depth_bias, going from 1.0 at the top to -1.0 at
// the bottom (the middle line is 0.0) In combination with the rotating plane, it should
// demonstrate how different depth_bias values interact with geometry.
fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_plugin(PolylinePlugin)
        .add_system(move_camera)
        .add_system(rotate_plane)
        .add_startup_system(setup)
        .run();
}

#[derive(Component)]
struct Rotating(f64);

fn rotate_plane(time: Res<Time>, mut animated: Query<(&mut Transform, &Rotating)>) {
    let time = time.elapsed_seconds_f64();
    for (mut trans, Rotating(period)) in animated.iter_mut() {
        let angle = time % period / period * TAU64;
        let rot = Quat::from_rotation_y(angle as f32);
        trans.rotation = Quat::from_rotation_y(TAU / 4.1) * rot;
    }
}

fn move_camera(input: Res<Input<KeyCode>>, mut camera: Query<&mut Transform, With<Camera>>) {
    if let Ok(mut camera_transform) = camera.get_single_mut() {
        let trans = &mut camera_transform.translation;
        let go_forward = input.any_pressed([KeyCode::Up, KeyCode::I, KeyCode::W]);
        let go_backward = input.any_pressed([KeyCode::Down, KeyCode::K, KeyCode::S]);
        if go_forward && trans.x > 10.0 {
            trans.x -= 2.0;
        } else if go_backward && trans.x < 500.0 {
            trans.x += 2.0;
        }
    }
}
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut pbr_materials: ResMut<Assets<StandardMaterial>>,
    mut polylines: ResMut<Assets<Polyline>>,
    mut materials: ResMut<Assets<PolylineMaterial>>,
) {
    commands
        .spawn(Camera3dBundle::default())
        .insert(Transform::from_xyz(100.0, 0.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(shape::Box::new(0.01, 100.0, 10000.0).into()),
            material: pbr_materials.add(Color::WHITE.into()),
            ..default()
        },
        Rotating(30.0),
    ));
    let top = Vec3::Y * 100.0;
    let bottom = Vec3::Y * -100.0;
    // Show the middle as a vertical red bar.
    commands.spawn(PolylineBundle {
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
    // Draw from bottom to top, red to purple, -1.0 to 1.0 horizontal lines
    for i in 0..100 {
        let bias = (i as f32) / 50.0 - 1.0;
        let left = Vec3::new(0.0, bias * 35.0, -500.0);
        let right = Vec3::new(0.0, bias * 35.0, 500.0);
        commands.spawn(PolylineBundle {
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
