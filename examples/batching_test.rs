use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};
use bevy_polyline::{prelude::*, PolylineSettings};

const SIDE_LEN: i32 = 60;
const POLY_CUBE_SIDE_LEN: i32 = 3;

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                present_mode: bevy::window::PresentMode::Immediate,
                ..default()
            },
            ..default()
        }))
        .add_plugin(PolylinePlugin)
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_startup_system(setup)
        .add_system(toggle_batching)
        .add_system(rotator_system)
        .run();
}

fn toggle_batching(mut settings: ResMut<PolylineSettings>, input: Res<Input<KeyCode>>) {
    if input.just_pressed(KeyCode::Space) {
        settings.batching_enabled = !settings.batching_enabled;
        info!("Batching enabled: {}", settings.batching_enabled);
    }
}

fn setup(mut commands: Commands, mut polyline_materials: ResMut<Assets<PolylineMaterial>>) {
    let n_entities = SIDE_LEN.pow(3);
    let n_lines = n_entities * POLY_CUBE_SIDE_LEN.pow(3);
    info!("Rendering {n_entities} polyline entities, with a total of {n_lines} lines.");

    let material = polyline_materials.add(PolylineMaterial {
        width: 4.0,
        color: Color::WHITE,
        perspective: true,
        ..Default::default()
    });

    let mut list = Vec::new();

    for x in 0..SIDE_LEN {
        for y in 0..SIDE_LEN {
            for z in 0..SIDE_LEN {
                let x = (x * 2 - SIDE_LEN / 2) as f32;
                let y = (y * 2 - SIDE_LEN / 2) as f32;
                let z = (z * 2 - SIDE_LEN / 2) as f32;

                let poly_cube_origin = Vec3::new(x, y, z);

                list.push(PolylineBundle {
                    polyline: Polyline::new(
                        (0..POLY_CUBE_SIDE_LEN.pow(3))
                            .map(|i| {
                                let ix = i % POLY_CUBE_SIDE_LEN;
                                let iy = i / POLY_CUBE_SIDE_LEN % POLY_CUBE_SIDE_LEN;
                                let iz = i / POLY_CUBE_SIDE_LEN.pow(2) % POLY_CUBE_SIDE_LEN;
                                Vec3::new(ix as f32, iy as f32, iz as f32)
                                    / (POLY_CUBE_SIDE_LEN as f32)
                                    + poly_cube_origin
                            })
                            .collect(),
                    ),
                    material: material.clone(),
                    ..Default::default()
                });
            }
        }
    }
    commands.spawn_batch(list);

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
            (4.0 * std::f32::consts::PI / 50.0) * time.delta_seconds(),
        )) * *transform;
    }
}
