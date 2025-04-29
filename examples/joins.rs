use std::f32::consts::TAU;

use bevy::{
    color::palettes::css::{BLUE, RED},
    prelude::*,
};
use bevy_polyline::prelude::*;

const NUM_STEPS: u16 = 8;
const RADIUS: f32 = 1.0;
const WIDTH: f32 = 60.0;

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
    for (joins, offset, color) in [(false, -1.0, RED), (true, 1.0, BLUE)] {
        let center = Vec3 {
            x: 1.5 * offset,
            y: 0.0,
            z: 0.0,
        };

        let max = f32::from(NUM_STEPS);
        let vertices = (0u16..NUM_STEPS + 1)
            .map(|t| {
                let angle = f32::from(t) / max * TAU;
                Vec3 {
                    x: angle.cos() * RADIUS,
                    y: angle.sin() * RADIUS,
                    z: 0.0,
                } + center
            })
            .collect();

        commands.spawn(PolylineBundle {
            polyline: PolylineHandle(polylines.add(Polyline { vertices })),
            material: PolylineMaterialHandle(polyline_materials.add(PolylineMaterial {
                width: WIDTH,
                color: color.into(),
                perspective: false,
                joins,
                ..default()
            })),
            ..default()
        });
    }

    // camera
    commands.spawn((
        Camera3d::default(),
        Camera {
            hdr: true,
            ..default()
        },
        Msaa::Sample4,
        Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
