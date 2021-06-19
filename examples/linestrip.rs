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
                width: 1.0,
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
        mesh: asset_server.load("models/pix4d_quarry_2.glb#Mesh0/Primitive0"),
        material: standard_materials.add(StandardMaterial {
            base_color: Color::rgba(1.0, 1.0, 1.0, 0.5),
            ..Default::default()
        }),
        transform: Transform::from_xyz(-1.0, 0.0, 0.0),
        ..Default::default()
    });

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
        transform: Transform::from_xyz(700.0, -200.0, 50.0).looking_at(center, Vec3::Y),
        ..PerspectiveCameraBundle::new_3d()
    });
    // .insert(Rotates);
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
