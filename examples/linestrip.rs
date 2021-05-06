use bevy::{
    pbr::PointLightBundle,
    prelude::*,
    render::{
        pass::{
            LoadOp, Operations, PassDescriptor, RenderPassDepthStencilAttachment, TextureAttachment,
        },
        render_graph::{base, RenderGraph, WindowSwapChainNode, WindowTextureNode},
    },
};
use bevy_poly_line::{PolyLine, PolyLineBundle, PolyLineNode, PolyLinePlugin};

mod node {
    pub const POLY_LINE_NODE: &str = "poly_line_node";
}

fn main() {
    let mut app = App::build();

    app.insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_plugin(PolyLinePlugin {})
        .add_startup_system(setup.system());

    {
        let world = app.world_mut().cell();
        let mut render_graph = world.get_resource_mut::<RenderGraph>().unwrap();
        let msaa = world.get_resource::<Msaa>().unwrap();

        let poly_line_node = PolyLineNode::new(PassDescriptor {
            color_attachments: vec![msaa.color_attachment(
                TextureAttachment::Input("color_attachment".to_string()),
                TextureAttachment::Input("color_resolve_target".to_string()),
                Operations {
                    load: LoadOp::Load,
                    store: true,
                },
            )],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                attachment: TextureAttachment::Input("depth".to_string()),
                depth_ops: Some(Operations {
                    load: LoadOp::Load,
                    store: true,
                }),
                stencil_ops: None,
            }),
            sample_count: msaa.samples,
        });
        render_graph.add_node(node::POLY_LINE_NODE, poly_line_node);

        // Make this run after MainPass
        render_graph
            .add_node_edge(base::node::MAIN_PASS, node::POLY_LINE_NODE)
            .unwrap();

        if msaa.samples > 1 {
            render_graph
                .add_slot_edge(
                    base::node::MAIN_SAMPLED_COLOR_ATTACHMENT,
                    WindowTextureNode::OUT_TEXTURE,
                    node::POLY_LINE_NODE,
                    "color_attachment",
                )
                .unwrap();
        }
        render_graph
            .add_slot_edge(
                base::node::PRIMARY_SWAP_CHAIN,
                WindowSwapChainNode::OUT_TEXTURE,
                node::POLY_LINE_NODE,
                if msaa.samples > 1 {
                    "color_resolve_target"
                } else {
                    "color_attachment"
                },
            )
            .unwrap();
        render_graph
            .add_slot_edge(
                base::node::MAIN_DEPTH_TEXTURE,
                WindowTextureNode::OUT_TEXTURE,
                node::POLY_LINE_NODE,
                "depth",
            )
            .unwrap();

        // Make UiPass run after this
        // only way to currently detect if ui node in graph without adding a ui feature or depending on bevy_ui
        if render_graph.get_node_id("ui_node").is_ok() {
            render_graph
                .add_node_edge(node::POLY_LINE_NODE, "ui_node")
                .unwrap();
        }
    }

    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..Default::default()
    });
    // cube
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..Default::default()
    });

    // commands.spawn_bundle(PolyLineBundle {
    //     poly_line: PolyLine {
    //         vertices: vec![
    //             Vec3::new(0., 0., 0.),
    //             Vec3::new(0., 1., 0.),
    //             Vec3::new(0., 1., 1.),
    //             Vec3::new(0., 0., 0.),
    //         ],
    //     },
    //     ..Default::default()
    // });

    // light
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..Default::default()
    });

    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..PerspectiveCameraBundle::new_3d()
    });
}
