#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

use bevy::{prelude::*, asset::embedded_asset};
use material::PolylineMaterialPlugin;
use polyline::{PolylineBasePlugin, PolylineRenderPlugin};

pub mod material;
pub mod polyline;

pub mod prelude {
    pub use crate::material::PolylineMaterial;
    pub use crate::polyline::{Polyline, PolylineBundle};
    pub use crate::PolylinePlugin;
}
pub struct PolylinePlugin;

impl Plugin for PolylinePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        embedded_asset!(app, "src\\", "shaders\\polyline.wgsl");

        app.add_plugins((
            PolylineBasePlugin,
            PolylineRenderPlugin,
            PolylineMaterialPlugin,
        ));
    }
}
