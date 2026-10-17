//! Strongly typed runtime model data structures optimized for mobile GPU staging and rendering.

use bytemuck::{Pod, Zeroable};

/// Axis-aligned bounding box.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct RuntimeBounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl Default for RuntimeBounds {
    fn default() -> Self {
        Self {
            min: [f32::MAX; 3],
            max: [f32::MIN; 3],
        }
    }
}

pub use open_nfs_core::math::RuntimeVertex;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct RuntimeSubmesh {
    pub material_id: u32,
    pub index_offset: u32,
    pub index_count: u32,
    pub shader_flags: u32,
    pub bounds: RuntimeBounds,
}

/// A logical vehicle part or sub-component containing vertices and draw calls.
#[derive(Debug, Clone, PartialEq)]
pub struct RuntimePart {
    pub name: String,
    pub lod_level: u16,
    pub part_hash: u32,
    pub bounds: RuntimeBounds,
    pub vertices: Vec<RuntimeVertex>,
    pub indices: Vec<u16>,
    pub submeshes: Vec<RuntimeSubmesh>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RuntimeModel {
    pub source_sha256: [u8; 32],
    pub parts: Vec<RuntimePart>,
}

impl RuntimeModel {
    pub fn find_part(&self, name: &str) -> Option<&RuntimePart> {
        self.parts.iter().find(|p| p.name == name)
    }

    pub fn total_vertices(&self) -> usize {
        self.parts.iter().map(|p| p.vertices.len()).sum()
    }

    pub fn total_indices(&self) -> usize {
        self.parts.iter().map(|p| p.indices.len()).sum()
    }
}
