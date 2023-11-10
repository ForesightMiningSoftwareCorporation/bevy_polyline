#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

use bevy::prelude::*;
use material::PolylineMaterialPlugin;
use polyline::{PolylineRenderPlugin};

pub mod material;
pub mod polyline;

pub mod prelude {
    pub use crate::material::PolylineMaterial;
    pub use crate::polyline::{Polyline, PolylineBundle};
    pub use crate::PolylinePlugin;
}

pub const SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(12823766040132746065);

pub struct PolylinePlugin;

impl Plugin for PolylinePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            PolylineRenderPlugin,
            PolylineMaterialPlugin,
        ));
    }
}
