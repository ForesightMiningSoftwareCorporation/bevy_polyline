# Bevy Polyline

Bevy Polyline is a plugin for [Bevy Engine](https://bevyengine.org/) that adds instanced rendering of `Polyline`s. The plugin comes courtesy of Foresight Mining Software Corporation who sponsor its creation and maintenance.

Here's quick demo:

![nbody demo](nbody.gif)

## Implementation

Bevy Polyline closely mimics the way `Mesh`es are rendered in Bevy. It works internally by passing a minimal Instance Buffer to the GPU, containing only the line segment endpoints and then completely determines all vertex positions within the vertex shader, such that the triangles form a line that is rotated around it's longitudinal axis to face towards the camera. The shader code is based on this great tutorial by Rye Terrell.

Bevy Polylines, through `PolylineBundle` uses the `Draw`, `RenderPipelines` and `MainPass` components that `PbrBundle` does, so that it can be rendered by the `MainPass`. This means it fully respects depth and writes depth just like `Mesh`es. If this is not desired, it can be used in a custom pass, similar to `PbrBundle`, by removing the `MainPass` component and setting up a separate `PassNode` with a custom `WorldQuery`.

## Examples
There are two examples, linestrip demonstrates how to make a very basic static Polyline. nbody (shown in the above demo) demonstrates how to do updateable `Polyline`s, by changing the vertices of a `Polyline`.

## Usage
Usage of Bevy Polyline is quite simple. First add it to your `Cargo.toml`. It has not been published to crates.io just yet:

```toml
[dependencies]
bevy_polyline = { git = "https://github.com/ForesightMiningSoftwareCorporation/bevy_polyline.git", branch = "main" }
```

You add it as a plugin to your app:
```rust
    app.add_plugin(PolylinePlugin);
```

And then you can add some Polylines through PolylineBundle
```rust
    commands.spawn_bundle(PolylineBundle {
        polyline: Polyline {
            vertices: vec![
                Vec3::new(-0.5, 0.0, -0.5),
                Vec3::new(0.5, 0.0, -0.5),
                Vec3::new(0.5, 1.0, -0.5),
                Vec3::new(-0.5, 1.0, -0.5),
                Vec3::new(-0.5, 1.0, 0.5),
                Vec3::new(0.5, 1.0, 0.5),
                Vec3::new(0.5, 0.0, 0.5),
                Vec3::new(-0.5, 0.0, 0.5),
            ],
        },
        material: polyline_materials.add(PolylineMaterial {
            width: 5.0,
            color: Color::RED,
            perspective: false,
        }),
        ..Default::default()
    });
```

## Transform
`Polyline`s respect positioning through `GlobalTransform`, so you can position them directly, or through the use of a `Transform` hierarchy.

## PolylineMaterial
Currently the main way of customizing a `Polyline` is by changing the `PolylineMaterial`, which, as can be seen above, has fields for `width`, `color` and `perspective`. `width` directly correlates to screen pixels in non-perspective mode. In `perspective` mode `width` gets divided by the w component of the homogeneous coordinate, meaning it corresponds to screen pixels at the near plane and becomes progressively smaller further away.

## Shaders
For more significant customization, you have to make a custom shader, although it's likely we'll add more shaders in the future. The current version only implements line strips (i.e. `PolyLine`s rendered as connected line segments) with miter joins and no caps. Line lists should be added soon, as will some other types of joins and caps from Rye Terrells article.

Due to the nature of its instanced rendering, Bevy Polyline comes with fairly specific shaders. You can still replace these with custom ones, but you will have to keep a good chunk of the shader in tact if you want to use Bevy Polyline's way of creating the line triangles.

## Aliasing/shimmering
Bevy Polyline does some work to reduce aliasing, by implementing the line thinness fade from https://acegikmo.com/shapes/docs/#anti-aliasing. But if your line segments are very short, you will still see shimmering, caused by triangles < 1 pixel in size. In the `nbody` example, this is reduced by only adding segments of a minimum length and something similar may be possible in your use case.

## Performance
Due to instancing, Bevy Polyline only makes two drawcalls per `PolyLine`, one for the line segments and one for the miter joins. We've tested the `nbody` demo at some 500 lines with 4096 segments being updated every frame (in addition to a 4th order yoshida integrator for the nbody simulation) running at 60fps. There is still some room for performance optimisation, particularly reducing to one drawcall per `Polyline` (depending on join and cap types) and more efficient updates of the instance buffer for updated lines.

## Bevy version support
Due to a one line bug (+ removal of a `use` statement) in Bevy that was [found and fixed](https://github.com/bevyengine/bevy/pull/2126) after the release of Bevy 0.5.0, Bevy Polyline can currently only support Bevy *main*. If you need Bevy Polyline to work with 0.5.0, you need to make this two line patch to a custom Bevy version:

```rust
diff --git a/crates/bevy_render/src/pipeline/pipeline_compiler.rs b/crates/bevy_render/src/pipeline/pipeline_compiler.rs
index d8cc2575..84753b02 100644
--- a/crates/bevy_render/src/pipeline/pipeline_compiler.rs
+++ b/crates/bevy_render/src/pipeline/pipeline_compiler.rs
@@ -1,6 +1,6 @@
 use super::{state_descriptors::PrimitiveTopology, IndexFormat, PipelineDescriptor};
 use crate::{
-    pipeline::{BindType, InputStepMode, VertexBufferLayout},
+    pipeline::{BindType, VertexBufferLayout},
     renderer::RenderResourceContext,
     shader::{Shader, ShaderError},
 };
@@ -205,7 +205,7 @@ impl PipelineCompiler {

         // the vertex buffer descriptor that will be used for this pipeline
         let mut compiled_vertex_buffer_descriptor = VertexBufferLayout {
-            step_mode: InputStepMode::Vertex,
+            step_mode: mesh_vertex_buffer_layout.step_mode,
             stride: mesh_vertex_buffer_layout.stride,
             ..Default::default()
         };
```

## Community Support
If you want some help using this plugin, you can find usually find the maintainer Jonas Matser (nickname: `mtsr`) in the Bevy Discord at https://discord.gg/bevy.

## Sponsors
The creation and maintenance of Bevy Polyline is sponsored by Foresight Mining Software Corporation.

<img src="fse.png" alt="Foresight Mining Software Corporation" width="240">
