//! `open-nfs-render` — Vulkan and wgpu graphics backend, camera, and pipeline.
//!
//! Provides hardware-accelerated 3D vehicle rendering, 2D mobile virtual controls overlay,
//! orbit camera manipulation, and surface lifecycle management conforming to Milestone M3.

#![deny(unsafe_code)]

pub mod camera;
pub mod capabilities;
pub mod context;
pub mod controls;
pub mod diagnostics;
pub mod mesh;
pub mod offscreen;
pub mod pipeline;
pub mod surface;
pub mod texture;

pub fn render_version() -> &'static str {
    "0.1.0"
}
