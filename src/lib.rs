#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

use bevy::{asset::embedded_asset, prelude::*};
use material::PolylineMaterialPlugin;
use polyline::{PolylineBasePlugin, PolylineRenderPlugin};

pub mod material;
pub mod polyline;

pub mod prelude {
    pub use crate::material::{PolylineMaterial, PolylineMaterialHandle};
    pub use crate::polyline::{Polyline, PolylineBundle, PolylineHandle};
    pub use crate::PolylinePlugin;
}
pub struct PolylinePlugin;

impl Plugin for PolylinePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        embedded_asset!(app, "shaders/polyline.wesl");

        app.add_plugins((
            PolylineBasePlugin,
            PolylineRenderPlugin,
            PolylineMaterialPlugin,
        ));
    }
}
