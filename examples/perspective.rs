use bevy::prelude::*;
use bevy_polyline::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PolylinePlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, (move_camera, toggle_perspective))
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
            color: Color::RED,
            perspective: true,
            ..default()
        }),
        ..default()
    });

    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 0.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
        camera: Camera {
            hdr: true,
            ..default()
        },
        ..default()
    });
}

fn move_camera(
    mut q: Query<&mut Transform, With<Camera>>,
    keyboard_input: Res<Input<KeyCode>>,
    time: Res<Time>,
) {
    let speed = 5.0;
    for mut t in &mut q {
        let mut dir = Vec3::ZERO;
        if keyboard_input.pressed(KeyCode::W) {
            dir.z -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::S) {
            dir.z += 1.0;
        }
        if keyboard_input.pressed(KeyCode::A) {
            dir.x -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::D) {
            dir.x += 1.0;
        }
        if keyboard_input.pressed(KeyCode::Q) {
            dir.y -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::E) {
            dir.y += 1.0;
        }
        t.translation += dir * time.delta_seconds() * speed;
    }
}

fn toggle_perspective(
    mut polyline_materials: ResMut<Assets<PolylineMaterial>>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    for (_, mut mat) in polyline_materials.iter_mut() {
        if keyboard_input.just_pressed(KeyCode::X) {
            mat.perspective = !mat.perspective;
        }
    }
}
