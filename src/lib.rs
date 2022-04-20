use bevy::{prelude::*, reflect::TypeUuid};
use material::PolylineMaterialPlugin;
use polyline::{PolylineBasePlugin, PolylineRenderPlugin};

pub mod material;
pub mod polyline;

pub mod prelude {
    pub use crate::material::PolylineMaterial;
    pub use crate::polyline::{Polyline, PolylineBundle};
    pub use crate::PolylinePlugin;
}

pub const FRAG_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 12823766040132746065);
pub const VERT_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 10060193527938109963);

pub struct PolylinePlugin;

impl Plugin for PolylinePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let mut shaders = app.world.get_resource_mut::<Assets<Shader>>().unwrap();
        shaders.set_untracked(
            FRAG_SHADER_HANDLE,
            Shader::from_glsl(
                include_str!("shaders/polyline.frag"),
                naga::ShaderStage::Fragment,
            ),
        );
        shaders.set_untracked(
            VERT_SHADER_HANDLE,
            Shader::from_glsl(
                include_str!("shaders/polyline.vert"),
                naga::ShaderStage::Vertex,
            ),
        );
        app.add_plugin(PolylineBasePlugin)
            .add_plugin(PolylineRenderPlugin)
            .add_plugin(PolylineMaterialPlugin);
    }
}
