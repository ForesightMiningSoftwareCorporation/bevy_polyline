use std::f32::consts::PI;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    math::Vec3A,
    prelude::*,
    window::PresentMode,
};
use bevy_polyline::prelude::*;

use lazy_static::*;
use rand::{prelude::*, Rng};
use ringbuffer::{ConstGenericRingBuffer, RingBufferExt, RingBufferWrite};

const NUM_BODIES: usize = 512;
const TRAIL_LENGTH: usize = 1024;
const MINIMUM_ANGLE: f32 = 1.48341872; // == acos(5 degrees)

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(Simulation {
            scale: 1e5,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                width: 1920.0,
                height: 1080.0,
                resizable: false,
                present_mode: PresentMode::Immediate,
                ..default()
            },
            ..default()
        }))
        .add_plugin(PolylinePlugin)
        .add_startup_system(setup)
        .add_system(nbody_system)
        .add_system(update_trails.after(nbody_system))
        .add_system(rotator_system)
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(LogDiagnosticsPlugin::default())
        .run();
}

fn setup(
    mut commands: Commands,
    mut polyline_materials: ResMut<Assets<PolylineMaterial>>,
    mut polylines: ResMut<Assets<Polyline>>,
) {
    let mut rng = StdRng::seed_from_u64(0);
    for _index in 0..NUM_BODIES {
        let r = rng.gen_range(2f32..800f32);
        let theta = rng.gen_range(0f32..2.0 * PI);
        let position = Vec3A::new(
            r * f32::cos(theta),
            rng.gen_range(-500f32..500f32),
            r * f32::sin(theta),
        );
        let size = rng.gen_range(50f32..1000f32);
        commands.spawn((
            Body {
                mass: size,
                position,
                velocity: position.cross(Vec3A::Y).normalize() * 0.00019,
                ..Default::default()
            },
            Trail(ConstGenericRingBuffer::<Vec3A, TRAIL_LENGTH>::new()),
            PolylineBundle {
                polyline: polylines.add(Polyline {
                    vertices: Vec::with_capacity(TRAIL_LENGTH),
                }),
                material: polyline_materials.add(PolylineMaterial {
                    width: (size * 0.1).powf(1.8),
                    color: Color::hsl(rng.gen_range(0.0..360.0), 1.0, rng.gen_range(0.4..2.0)),
                    perspective: true,
                    ..Default::default()
                }),
                ..Default::default()
            },
        ));
    }

    // camera
    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                hdr: true,
                ..default()
            },
            ..default()
        },
        bevy::core_pipeline::bloom::BloomSettings {
            intensity: 0.1,
            ..default()
        },
        Rotates,
    ));
}

/// this component indicates what entities should rotate
#[derive(Component)]
struct Rotates;

fn rotator_system(time: Res<Time>, mut query: Query<&mut Transform, With<Rotates>>) {
    for mut transform in query.iter_mut() {
        let t = time.elapsed_seconds();
        let r = 1100.0;
        *transform = Transform::from_xyz(
            r * f32::cos(t * 0.1),
            (t * 0.1).sin() * 2000.0,
            r * f32::sin(t * 0.1),
        )
        .looking_at(Vec3::ZERO, Vec3::Y);
    }
}

#[derive(Clone, Debug, Default, Component)]
struct Body {
    mass: f32,
    acceleration: Vec3A,
    velocity: Vec3A,
    position: Vec3A,
}

#[derive(Debug, Resource)]
struct Simulation {
    pub accumulator: f32,
    pub is_paused: bool,
    pub scale: f32,
    pub timestep: f32,
}

impl Default for Simulation {
    fn default() -> Simulation {
        Simulation {
            accumulator: 0.0,
            is_paused: false,
            scale: 5e4,
            timestep: 1. / 30.,
        }
    }
}

impl Simulation {
    fn update(&mut self, time: &Time) {
        if !self.is_paused {
            self.accumulator += time.delta_seconds();
        }
    }

    fn step(&mut self) -> Option<f32> {
        if !self.is_paused && self.accumulator > self.timestep {
            self.accumulator -= self.timestep;
            return Some(self.timestep * self.scale);
        }
        None
    }
}
#[derive(Component, Clone, Default, Debug)]
struct Trail(ConstGenericRingBuffer<Vec3A, TRAIL_LENGTH>);

const G: f32 = 6.674_30E-11;
const EPSILON: f32 = 1.;

fn nbody_system(
    time: Res<Time>,
    mut simulation: ResMut<Simulation>,
    mut query: Query<(Entity, &mut Body, &mut Trail, &Handle<Polyline>)>,
) {
    let mut bodies = query.iter_mut().collect::<Vec<_>>();
    // dbg!(&bodies);

    // Step simulation in fixed increments
    simulation.update(&*time);
    while let Some(dt) = simulation.step() {
        // Start substeps
        for substep in 0..3 {
            // Clear accelerations and update positions
            for (_, body, _, _) in bodies.iter_mut() {
                body.acceleration = Vec3A::ZERO;
                let dx = (*CS)[substep] * body.velocity * dt;
                body.position += dx;
            }

            // Update accelerations
            for index1 in 0..bodies.len() {
                let (bodies1, bodies2) = bodies.split_at_mut(index1 + 1);
                let (_, body1, _, _) = &mut bodies1[index1];
                for (_, body2, _, _) in bodies2.iter_mut() {
                    let offset = body2.position - body1.position;
                    let distance_squared = offset.length_squared();
                    let normalized_offset = offset / distance_squared.sqrt();

                    let da = (G * body2.mass / (distance_squared + EPSILON)) * normalized_offset;
                    body1.acceleration += da;
                    body2.acceleration -= da;
                }
            }

            // Update velocities
            for (_, body, _, _) in bodies.iter_mut() {
                let dv = (*DS)[substep] * body.acceleration * dt;
                body.velocity += dv;
                if substep == 2 {
                    let dx = *C4 * body.velocity * dt;
                    body.position += dx;
                }
            }
        }
    }
}

fn update_trails(
    mut polylines: ResMut<Assets<Polyline>>,
    mut query: Query<(&Body, &mut Trail, &Handle<Polyline>)>,
) {
    query.for_each_mut(|(body, mut trail, polyline)| {
        if let Some(position) = trail.0.back() {
            let last_vec = *position - body.position;
            let last_last_vec = if let Some(position) = trail.0.get(-2) {
                *position - body.position
            } else {
                last_vec
            };
            let gt_min_angle = last_vec.dot(last_last_vec) > MINIMUM_ANGLE;
            if gt_min_angle {
                trail.0.push(body.position);
                polylines.get_mut(polyline).unwrap().vertices =
                    trail.0.iter().map(|v| Vec3::from(*v)).collect()
            } else {
                // If the last point didn't actually add much of a curve, just overwrite it.
                if polylines.get_mut(polyline).unwrap().vertices.len() > 1 {
                    *trail.0.get_mut(-1).unwrap() = body.position;
                    *polylines
                        .get_mut(polyline)
                        .unwrap()
                        .vertices
                        .last_mut()
                        .unwrap() = body.position.into();
                }
            }
        } else {
            trail.0.push(body.position);
            polylines.get_mut(polyline).unwrap().vertices =
                trail.0.iter().map(|v| Vec3::from(*v)).collect()
        }
    });
}

lazy_static! {
    static ref W0: f32 = -2f32.cbrt() / (2f32 - 2f32.cbrt());
    static ref W1: f32 = 1f32 / (2f32 - 2f32.cbrt());
    static ref C1: f32 = *W1 / 2f32;
    static ref C2: f32 = (*W0 + *W1) / 2f32;
    static ref C3: f32 = *C2;
    static ref C4: f32 = *C1;
    static ref CS: [f32; 4] = [*C1, *C2, *C3, *C4];
    static ref D1: f32 = *W1;
    static ref D2: f32 = *W0;
    static ref D3: f32 = *D1;
    static ref DS: [f32; 3] = [*D1, *D2, *D3];
}
