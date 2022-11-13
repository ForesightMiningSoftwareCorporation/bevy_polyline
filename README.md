<div align="center">
    
# Bevy Polyline

**High performance instanced polyline rendering for bevy**
    
https://user-images.githubusercontent.com/2632925/164312056-2812d46c-6111-40b8-bbb2-087f7ee9afb2.mp4
    
[![crates.io](https://img.shields.io/crates/v/bevy_polyline)](https://crates.io/crates/bevy_polyline)
[![docs.rs](https://docs.rs/bevy_polyline/badge.svg)](https://docs.rs/bevy_polyline)
[![CI](https://github.com/ForesightMiningSoftwareCorporation/bevy_polyline/workflows/CI/badge.svg?branch=main)](https://github.com/ForesightMiningSoftwareCorporation/bevy_polyline/actions?query=workflow%3A%22CI%22+branch%3Amain)
[![Bevy tracking](https://img.shields.io/badge/Bevy%20tracking-main-lightblue)](https://github.com/bevyengine/bevy/blob/main/docs/plugins_guidelines.md#main-branch-tracking)
    
</div>

## About

Bevy Polyline is a plugin for [Bevy Engine](https://bevyengine.org/) that adds instanced rendering of `Polyline`s. The plugin comes courtesy of Foresight Mining Software Corporation who sponsor its creation and maintenance. Special thanks to [mtsr](https://github.com/mtsr) for the initial implementation of this plugin.

### Implementation

Bevy Polyline closely mimics the way `Mesh`es are rendered in Bevy. It works internally by passing a minimal Instance Buffer to the GPU, containing only the line segment endpoints and then completely determines all vertex positions within the vertex shader, such that the triangles form a line that is rotated around it's longitudinal axis to face towards the camera. The shader code is based on [this great tutorial by Rye Terrell](https://wwwtyro.net/2019/11/18/instanced-lines.html).

## Usage

See the `minimal` example for basic usage.

### Transform
`Polyline`s respect positioning through `GlobalTransform`, so you can position them directly, or through the use of a `Transform` hierarchy.

### PolylineMaterial
Currently the main way of customizing a `Polyline` is by changing the `PolylineMaterial`, which, as can be seen above, has fields for `width`, `color` and `perspective`. `width` directly correlates to screen pixels in non-perspective mode. In `perspective` mode `width` gets divided by the w component of the homogeneous coordinate, meaning it corresponds to screen pixels at the near plane and becomes progressively smaller further away.

### Aliasing/shimmering
Bevy Polyline does some work to reduce aliasing, by implementing the line thinness fade from https://acegikmo.com/shapes/docs/#anti-aliasing. But if your line segments are very short, you will still see shimmering, caused by triangles < 1 pixel in size. This can be reduced by only adding segments of a minimum length.

### Performance
Due to instancing, Bevy Polyline only makes one drawcall per `PolyLine`, one for the line segments ~~and one for the miter joins~~ (not currently enabled). We've tested the `nbody` demo at some 500 lines with 4096 segments being updated every frame (in addition to a 4th order Yoshida integrator for the nbody simulation) running at 60fps. There is still some room for performance optimization, particularly reducing to one drawcall per `Polyline` (depending on join and cap types) and more efficient updates of the instance buffer for updated lines.

## Bevy Version Support
We intend to track the `main` branch of Bevy. PRs supporting this are welcome!

| bevy | bevy_polyline |
| ---- | ------------- |
| 0.9  | 0.4           |
| 0.8  | 0.3           |
| 0.7  | 0.2           |
| 0.6  | 0.1           |

### Community Support
If you want some help using this plugin, you can ask in the Bevy Discord at https://discord.gg/bevy.

## License

bevy_polyline is free and open source! All code in this repository is dual-licensed under either:

* MIT License ([LICENSE-MIT](LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

at your option. This means you can select the license you prefer! This dual-licensing approach is the de-facto standard in the Rust ecosystem and there are very good reasons to include both.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

## Sponsors
The creation and maintenance of Bevy Polyline is sponsored by Foresight Mining Software Corporation.

<img src="https://user-images.githubusercontent.com/2632925/151242316-db3455d1-4934-4374-8369-1818daf512dd.png" alt="Foresight Mining Software Corporation" width="480">
