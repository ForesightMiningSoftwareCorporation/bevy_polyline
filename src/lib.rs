#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

use bevy::{asset::load_internal_asset, prelude::*};
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

pub const SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(12823766040132746065);

impl Plugin for PolylinePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
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
