use std::f32::consts::PI;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};
use bevy_polyline::{Polyline, PolylineBundle, PolylineMaterial, PolylinePlugin};

use lazy_static::*;
use rand::{prelude::*, Rng};
use ringbuffer::{ConstGenericRingBuffer, RingBufferExt, RingBufferWrite};

const NUM_BODIES: usize = 500;
const TRAIL_LENGTH: usize = 2048;
const MINIMUM_LINE_SEGMENT_LENGTH_SQUARED: f32 = 0.1;

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            vsync: true,
            ..Default::default()
        })
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(Simulation {
            scale: 1e5,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(PolylinePlugin)
        .add_startup_system(setup.system())
        .add_system(nbody_system.system())
        .add_system(rotator_system.system())
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
        let position = Vec3::new(
            r * f32::cos(theta),
            rng.gen_range(-5f32..5f32),
            r * f32::sin(theta),
        );
        let size = rng.gen_range(50f32..2000f32);
        commands
            .spawn_bundle((
                Body {
                    mass: size,
                    position,
                    velocity: position.cross(Vec3::Y).normalize() * 0.00018,
                    ..Default::default()
                },
                Trail(ConstGenericRingBuffer::<Vec3, TRAIL_LENGTH>::new()),
            ))
            .insert_bundle(PolylineBundle {
                polyline: polylines.add(Polyline {
                    vertices: vec![Vec3::ZERO; TRAIL_LENGTH],
                }),
                material: polyline_materials.add(PolylineMaterial {
                    width: size,
                    color: Color::hsl(rng.gen_range(160.0..250.0), 1.0, rng.gen_range(0.4..0.7)),
                    perspective: true,
                    ..Default::default()
                }),
                ..Default::default()
            });
    }

    // camera
    commands
        .spawn_bundle(PerspectiveCameraBundle::new_3d())
        .insert(Rotates);
}

/// this component indicates what entities should rotate
#[derive(Component)]
struct Rotates;

fn rotator_system(time: Res<Time>, mut query: Query<&mut Transform, With<Rotates>>) {
    for mut transform in query.iter_mut() {
        let t = time.seconds_since_startup() as f32;
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
    acceleration: Vec3,
    velocity: Vec3,
    position: Vec3,
}

#[derive(Debug)]
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
struct Trail(ConstGenericRingBuffer<Vec3, TRAIL_LENGTH>);

const G: f32 = 6.674_30E-11;
const EPSILON: f32 = 1.;

fn nbody_system(
    time: Res<Time>,
    mut simulation: ResMut<Simulation>,
    mut polylines: ResMut<Assets<Polyline>>,
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
                body.acceleration = Vec3::ZERO;
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

    // Update Trails
    query
        .iter_mut()
        .for_each(|(_entity, body, mut trail, polyline)| {
            if let Some(position) = trail.0.back() {
                if (*position - body.position).length_squared()
                    > MINIMUM_LINE_SEGMENT_LENGTH_SQUARED
                {
                    trail.0.push(body.position);
                    polylines.get_mut(polyline).unwrap().vertices = trail.0.to_vec();
                }
            } else {
                trail.0.push(body.position);
                polylines.get_mut(polyline).unwrap().vertices = trail.0.to_vec();
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
