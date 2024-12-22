use bevy::{color::palettes::css::RED, prelude::*};
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
        polyline: PolylineHandle(polylines.add(Polyline {
            vertices: vec![-Vec3::ONE, Vec3::ONE],
        })),
        material: PolylineMaterialHandle(polyline_materials.add(PolylineMaterial {
            width: 10.0,
            color: RED.into(),
            perspective: true,
            ..default()
        })),
        ..default()
    });

    // camera
    commands.spawn((
        Camera3d::default(),
        Msaa::Sample4,
        Transform::from_xyz(0.0, 0.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
        Camera {
            hdr: true,
            ..default()
        },
    ));
}

fn move_camera(
    mut q: Query<&mut Transform, With<Camera>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let speed = 5.0;
    for mut t in &mut q {
        let mut dir = Vec3::ZERO;
        if keyboard_input.pressed(KeyCode::KeyW) {
            dir.z -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyS) {
            dir.z += 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyA) {
            dir.x -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            dir.x += 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyQ) {
            dir.y -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyE) {
            dir.y += 1.0;
        }
        t.translation += dir * time.delta_secs() * speed;
    }
}

fn toggle_perspective(
    mut polyline_materials: ResMut<Assets<PolylineMaterial>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    for (_, mat) in polyline_materials.iter_mut() {
        if keyboard_input.just_pressed(KeyCode::KeyX) {
            mat.perspective = !mat.perspective;
        }
    }
}
