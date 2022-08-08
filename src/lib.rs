#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

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

pub const SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 12823766040132746065);

pub struct PolylinePlugin;

impl Plugin for PolylinePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let mut shaders = app.world.get_resource_mut::<Assets<Shader>>().unwrap();
        shaders.set_untracked(
            SHADER_HANDLE,
            Shader::from_wgsl(include_str!("shaders/polyline.wgsl")),
        );
        app.add_plugin(PolylineBasePlugin)
            .add_plugin(PolylineRenderPlugin)
            .add_plugin(PolylineMaterialPlugin);
    }
}
