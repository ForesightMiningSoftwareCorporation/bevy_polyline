#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, weak_handle, Handle};
use bevy_render::render_resource::Shader;
use clipping::PolylineClippingPlugin;
use material::PolylineMaterialPlugin;
use polyline::{PolylineBasePlugin, PolylineRenderPlugin};

pub mod clipping;
pub mod material;
pub mod polyline;

pub mod prelude {
    pub use crate::material::{PolylineMaterial, PolylineMaterialHandle};
    pub use crate::polyline::{Polyline, PolylineBundle, PolylineHandle};
    pub use crate::PolylinePlugin;
}
pub struct PolylinePlugin;

pub const SHADER_HANDLE: Handle<Shader> = weak_handle!("b180bfe9-10c8-48fe-b27a-dfa41436d7d0");

impl Plugin for PolylinePlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            SHADER_HANDLE,
            "shaders/polyline.wgsl",
            Shader::from_wgsl
        );

        app.add_plugins((
            PolylineBasePlugin,
            PolylineRenderPlugin,
            PolylineClippingPlugin,
            PolylineMaterialPlugin,
        ));
    }
}
