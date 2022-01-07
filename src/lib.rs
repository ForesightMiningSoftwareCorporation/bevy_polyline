use bevy::prelude::*;
use material::PolylineMaterial;
use polyline::Polyline;

pub mod material;
pub mod polyline;

pub const FRAG_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 4805239651767701046);
pub const VERT_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 1836745567947005696);

pub struct PolylinePlugin;

impl Plugin for PolylinePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let mut shaders = app.world.get_resource_mut::<Assets<Shader>>().unwrap();
        shaders.set_untracked(
            material::FRAG_SHADER_HANDLE,
            Shader::from_glsl(
                include_str!("render/polyline.frag"),
                naga::ShaderStage::Fragment,
            ),
        );
        shaders.set_untracked(
            material::VERT_SHADER_HANDLE,
            Shader::from_glsl(
                include_str!("render/polyline.vert"),
                naga::ShaderStage::Vertex,
            ),
        );
        app.add_asset::<PolylineMaterial>().add_asset::<Polyline>();
    }
}

#[derive(Bundle)]
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

impl Default for PolylineBundle {
    fn default() -> Self {
        Self {
            polyline: Default::default(),
            material: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            visibility: Default::default(),
            computed_visibility: Default::default(),
        }
    }
}
