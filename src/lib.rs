use bevy::{prelude::*, reflect::TypeUuid};
pub use material::PolylineMaterial;
use material::PolylineMaterialPlugin;
pub use polyline::Polyline;
use polyline::{PolylineBasePlugin, PolylineRenderPlugin};

pub mod material;
pub mod polyline;

pub const SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 12823766040132746065);

pub const MESH_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 11173244208177162208);

pub struct PolylinePlugin;

impl Plugin for PolylinePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let mut shaders = app.world.get_resource_mut::<Assets<Shader>>().unwrap();
        shaders.set_untracked(
            SHADER_HANDLE,
            Shader::from_wgsl(include_str!("shaders/polyline.wgsl")),
        );
        shaders.set_untracked(
            MESH_SHADER_HANDLE,
            Shader::from_wgsl(include_str!("shaders/mesh_polyline.wgsl")),
        );
        app.add_plugin(PolylineBasePlugin)
            .add_plugin(PolylineRenderPlugin)
            .add_plugin(PolylineMaterialPlugin);
    }
}

#[derive(Bundle, Default)]
pub struct PolylineBundle {
    pub polyline: Handle<Polyline>,
    pub material: Handle<PolylineMaterial>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    /// User indication of whether an entity is visible
    pub visibility: Visibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub computed_visibility: ComputedVisibility,
}
