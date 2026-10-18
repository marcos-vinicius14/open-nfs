//! Persistent GPU vertex and index buffers for vehicle geometry.
//!
//! Conforms to REQ-M3-03, REQ-M3-04, and Section 3 of M3 guidelines:
//! Persistent GPU buffers, zero-allocation per frame, 36-byte vertex layout.

use open_nfs_asset_runtime::model::{RuntimeBounds, RuntimeModel, RuntimeSubmesh, RuntimeVertex};
use wgpu::util::DeviceExt;

/// Persistent GPU mesh containing vertex and index buffers with submesh ranges.
#[derive(Debug)]
pub struct GpuMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_vertices: u32,
    pub num_indices: u32,
    pub submeshes: Vec<RuntimeSubmesh>,
    pub bounds: RuntimeBounds,
}

impl GpuMesh {
    /// Create a GPU mesh from raw vertex and index slices.
    pub fn new(
        device: &wgpu::Device,
        label: &str,
        vertices: &[RuntimeVertex],
        indices: &[u32],
        submeshes: Vec<RuntimeSubmesh>,
        bounds: RuntimeBounds,
    ) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{label} Vertex Buffer")),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{label} Index Buffer")),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            vertex_buffer,
            index_buffer,
            num_vertices: vertices.len() as u32,
            num_indices: indices.len() as u32,
            submeshes,
            bounds,
        }
    }

    /// Assemble and upload a full stock car (LOD 0) from a `RuntimeModel`.
    pub fn from_runtime_model(device: &wgpu::Device, model: &RuntimeModel, label: &str) -> Self {
        let (vertices, indices, submeshes) = model.assemble_stock_car();

        let mut bounds = RuntimeBounds::default();
        for v in &vertices {
            for i in 0..3 {
                bounds.min[i] = bounds.min[i].min(v.position[i]);
                bounds.max[i] = bounds.max[i].max(v.position[i]);
            }
        }

        Self::new(device, label, &vertices, &indices, submeshes, bounds)
    }

    /// Bind vertex and index buffers to the render pass.
    pub fn bind<'a>(&'a self, rpass: &mut wgpu::RenderPass<'a>) {
        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
    }

    /// Draw all indices in a single draw call.
    pub fn draw_all<'a>(&'a self, rpass: &mut wgpu::RenderPass<'a>) {
        if self.num_indices > 0 {
            rpass.draw_indexed(0..self.num_indices, 0, 0..1);
        }
    }

    /// Draw a specific submesh by index.
    pub fn draw_submesh<'a>(&'a self, rpass: &mut wgpu::RenderPass<'a>, submesh_index: usize) {
        if let Some(submesh) = self.submeshes.get(submesh_index) {
            let start = submesh.index_offset;
            let end = start + submesh.index_count;
            if end <= self.num_indices {
                rpass.draw_indexed(start..end, 0, 0..1);
            }
        }
    }
}
