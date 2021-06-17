use bevy::{pbr::PointLightBundle, prelude::*};
use bevy_polyline::{
    pipeline::{new_polyline_pbr_pipeline, new_polyline_pipeline},
    Polyline, PolylineBundle, PolylineMaterial, PolylineMesh, PolylinePbrBundle,
    PolylinePbrMaterial, PolylinePlugin,
};

fn main() {
    let mut app = App::build();

    app.insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_plugin(PolylinePlugin)
        .add_startup_system(setup.system())
        .add_system(rotator_system.system());

    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut polyline_materials: ResMut<Assets<PolylineMaterial>>,
    mut polyline_pbr_materials: ResMut<Assets<PolylinePbrMaterial>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands
        .spawn_bundle(PolylinePbrBundle {
            polyline: Polyline {
                vertices: vec![Vec3::new(1.0, 0.0, 0.0), Vec3::new(2.0, 2.0, 0.0)],
                ..Default::default()
            },
            render_pipelines: RenderPipelines {
                pipelines: vec![new_polyline_pbr_pipeline(true)],
                ..Default::default()
            },
            polyline_pbr_material: polyline_pbr_materials.add(PolylinePbrMaterial {
                width: 850.0,
                perspective: true,
                base_color: Color::WHITE,
                ..Default::default()
            }),
            ..Default::default()
        })
        .insert(PolylineMesh {
            mesh: Some(
                asset_server.load::<Mesh, _>("models/capped_half_cylinder2.glb#Mesh0/Primitive0"),
            ),
        });

    commands.spawn_bundle(PbrBundle {
        mesh: asset_server.load("models/capped_cylinder2.glb#Mesh0/Primitive0"),
        material: standard_materials.add(StandardMaterial {
            base_color: Color::rgb(1.0, 1.0, 1.0),
            ..Default::default()
        }),
        transform: Transform::from_xyz(-1.0, 0.0, 0.0),
        ..Default::default()
    });

    // commands.spawn_bundle(PolylineBundle {
    //     polyline: Polyline {
    //         vertices: vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(5.0, 5.0, 0.0)],
    //         ..Default::default()
    //     },
    //     render_pipelines: RenderPipelines {
    //         pipelines: vec![new_polyline_pipeline(true)],
    //         ..Default::default()
    //     },
    //     polyline_material: polyline_materials.add(PolylineMaterial {
    //         width: 250.0,
    //         perspective: false,
    //         color: Color::RED,
    //         ..Default::default()
    //     }),
    //     transform: Transform::from_xyz(0.0, 0.0, 0.0),
    //     ..Default::default()
    // });

    // commands.spawn_bundle(PolylinePbrBundle {
    //     polyline: Polyline {
    //         vertices: vec![
    //             Vec3::new(-0.5, 0.0, -0.5),
    //             Vec3::new(0.5, 0.0, -0.5),
    //             Vec3::new(0.5, 1.0, -0.5),
    //             Vec3::new(-0.5, 1.0, -0.5),
    //             Vec3::new(-0.5, 1.0, 0.5),
    //             Vec3::new(0.5, 1.0, 0.5),
    //             Vec3::new(0.5, 0.0, 0.5),
    //             Vec3::new(-0.5, 0.0, 0.5),
    //         ],
    //         ..Default::default()
    //     },
    //     render_pipelines: RenderPipelines {
    //         pipelines: vec![new_polyline_pbr_pipeline(true)],
    //         ..Default::default()
    //     },
    //     polyline_pbr_material: polyline_pbr_materials.add(PolylinePbrMaterial {
    //         width: 15.0,
    //         perspective: false,
    //         base_color: Color::WHITE,
    //         ..Default::default()
    //     }),
    //     ..Default::default()
    // });

    // commands.spawn_bundle(PbrBundle {
    //     mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
    //     material: standard_materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
    //     ..Default::default()
    // });
    // cube
    // commands.spawn_bundle(PbrBundle {
    //     mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
    //     material: standard_materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
    //     transform: Transform::from_xyz(0.0, 0.5, 0.0),
    //     ..Default::default()
    // });

    // light
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(0.0, 10.0, 20.0),
        point_light: PointLight {
            // color: (),
            // intensity: (),
            range: 1000.0,
            // radius: (),
            ..Default::default()
        },
        ..Default::default()
    });

    // camera
    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_xyz(0.0, 10.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..PerspectiveCameraBundle::new_3d()
        })
        .insert(Rotates);
    // });
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
