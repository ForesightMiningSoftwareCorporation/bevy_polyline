use std::sync::Arc;

use bevy::{
    core::FixedTimestep,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    pbr::PointLightBundle,
    prelude::*,
    render::mesh::{Indices, VertexAttributeValues},
    tasks::{AsyncComputeTaskPool, Task},
};
use bevy_mod_raycast::{ray_mesh_intersection, Intersection, Ray3d};
use bevy_polyline::{
    pipeline::{new_polyline_pbr_pipeline, new_polyline_pipeline},
    Polyline, PolylineBundle, PolylineMaterial, PolylineMesh, PolylinePbrBundle,
    PolylinePbrMaterial, PolylinePlugin,
};
use futures_lite::future;

fn main() {
    let mut app = App::build();

    app.insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_plugin(PolylinePlugin)
        .add_startup_system(setup.system())
        .add_system(spawn_borehole_tasks.system())
        .add_system(handle_borehole_tasks.system())
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(0.5))
                .with_system(count_boreholes.system()),
        )
        .add_system(rotator_system.system())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(LogDiagnosticsPlugin::default());

    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut polyline_materials: ResMut<Assets<PolylineMaterial>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands
        .spawn_bundle(PbrBundle {
            mesh: asset_server.load("models/pix4d_quarry_2.glb#Mesh0/Primitive0"),
            material: standard_materials.add(StandardMaterial {
                base_color: Color::rgba(1.0, 1.0, 1.0, 0.8),
                ..Default::default()
            }),
            transform: Transform::from_xyz(-1.0, 0.0, 0.0),
            ..Default::default()
        })
        .insert(RaycastTodo);

    // commands.spawn_scene(asset_server.load("models/pix4d_quarry_2.glb#Scene0"));

    // commands.spawn_bundle(PbrBundle {
    //     mesh: asset_server.load("models/capped_cylinder2.glb#Mesh0/Primitive0"),
    //     material: standard_materials.add(StandardMaterial {
    //         base_color: Color::rgb(1.0, 1.0, 1.0),
    //         ..Default::default()
    //     }),
    //     transform: Transform::from_xyz(-1.0, 0.0, 0.0),
    //     ..Default::default()
    // });

    // light
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(0.0, 10.0, 0.0),
        point_light: PointLight {
            intensity: 40000.0,
            range: 100000.0,
            // radius: (),
            ..Default::default()
        },
        ..Default::default()
    });

    // camera
    let center = Vec3::new(0.0, -200.0, 0.0);
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(700.0, -200.0, 200.0).looking_at(center, Vec3::Y),
        ..PerspectiveCameraBundle::new_3d()
    });
    // .insert(Rotates);
}

struct MeshData {
    indices: Vec<u32>,
    positions: Vec<[f32; 3]>,
}

fn spawn_borehole_tasks(
    mut commands: Commands,
    thread_pool: Res<AsyncComputeTaskPool>,
    meshes: Res<Assets<Mesh>>,
    query: Query<(Entity, &GlobalTransform, &Handle<Mesh>), (With<RaycastTodo>)>,
) {
    query.for_each(|(entity, transform, mesh_handle)| {
        if let Some(mesh) = meshes.get(mesh_handle) {
            let mesh_to_world = transform.compute_matrix();

            if let Some(VertexAttributeValues::Float32x3(positions)) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION).cloned()
            {
                if let Some(Indices::U32(indices)) = mesh.indices().cloned() {
                    // if let Some(VertexAttributeValues::Float32x3(normals)) =
                    //     mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
                    // {
                    let mesh_data = Arc::new(MeshData { indices, positions });
                    for x in (-1000..1000).step_by(10) {
                        for z in (-500..500).step_by(10) {
                            let mesh_data = mesh_data.clone();
                            let from = Vec3::new(x as f32, 0.0, z as f32);

                            let task = thread_pool.spawn(async move {
                                let ray = Ray3d::new(from, -Vec3::Y);
                                let intersection = ray_mesh_intersection(
                                    &mesh_to_world,
                                    &mesh_data.positions,
                                    None,
                                    &ray,
                                    Some(&mesh_data.indices),
                                );

                                intersection
                            });

                            commands.spawn().insert(task);
                        }
                    }
                    // }
                }
            }

            commands.entity(entity).remove::<RaycastTodo>();
        };
    });
}

fn handle_borehole_tasks(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Task<Option<Intersection>>)>,
    mut polyline_pbr_materials: ResMut<Assets<PolylinePbrMaterial>>,
    asset_server: Res<AssetServer>,
) {
    for (entity, mut task) in query.iter_mut() {
        if let Some(result) = future::block_on(future::poll_once(&mut *task)) {
            if let Some(intersection) = result {
                // Add our new PbrBundle of components to our tagged entity
                commands
                    .entity(entity)
                    .insert_bundle(PolylinePbrBundle {
                        polyline: Polyline {
                            vertices: (0..40)
                                .step_by(1)
                                .map(|depth| {
                                    intersection.position() + Vec3::new(0.0, -depth as f32, 0.0)
                                })
                                .collect::<Vec<_>>(),
                            ..Default::default()
                        },
                        render_pipelines: RenderPipelines {
                            pipelines: vec![new_polyline_pbr_pipeline(true)],
                            ..Default::default()
                        },
                        polyline_pbr_material: polyline_pbr_materials.add(PolylinePbrMaterial {
                            width: 0.15,
                            perspective: true,
                            base_color: Color::RED,
                            ..Default::default()
                        }),
                        ..Default::default()
                    })
                    .insert(PolylineMesh {
                        mesh: Some(
                            asset_server.load::<Mesh, _>(
                                "models/capped_half_cylinder2.glb#Mesh0/Primitive0",
                            ),
                        ),
                    });
            }

            // Task is complete, so remove task component from entity
            commands
                .entity(entity)
                .remove::<Task<Option<Intersection>>>();
        }
    }
}

fn count_boreholes(query: Query<(), With<PolylineMesh>>) {
    dbg!(query.iter().count());
}

/// this component indicates what entities should rotate
struct Rotates;

fn rotator_system(time: Res<Time>, mut query: Query<&mut Transform, With<Rotates>>) {
    for mut transform in query.iter_mut() {
        *transform = Transform::from_rotation(Quat::from_rotation_y(
            (4.0 * std::f32::consts::PI / 20.0) * time.delta_seconds(),
        )) * *transform;
    }
}

struct RaycastTodo;
