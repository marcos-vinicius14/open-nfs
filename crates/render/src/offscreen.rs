//! Headless offscreen renderer for automated integration tests and screenshot export.
//!
//! Enables verification of the complete graphics pipeline without requiring an active window or physical display.

use crate::camera::OrbitCamera;
use crate::context::GpuContext;
use crate::controls::TouchManager;
use crate::mesh::GpuMesh;
use crate::pipeline::{UiPipeline, VehiclePipeline};
use crate::surface::DEPTH_FORMAT;
use crate::texture::TextureManager;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Headless offscreen renderer executing on any Vulkan/wgpu device.
pub struct OffscreenRenderer {
    pub width: u32,
    pub height: u32,
    pub color_texture: wgpu::Texture,
    pub color_view: wgpu::TextureView,
    pub depth_texture: wgpu::Texture,
    pub depth_view: wgpu::TextureView,
    pub vehicle_pipeline: VehiclePipeline,
    pub ui_pipeline: UiPipeline,
    pub format: wgpu::TextureFormat,
}

impl OffscreenRenderer {
    pub fn new(context: &GpuContext, width: u32, height: u32) -> Self {
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let color_texture = context.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Offscreen Color Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let depth_texture = context.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Offscreen Depth Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let vehicle_pipeline = VehiclePipeline::new(&context.device, format, Some(DEPTH_FORMAT));
        let ui_pipeline = UiPipeline::new(&context.device, format, Some(DEPTH_FORMAT));

        Self {
            width,
            height,
            color_texture,
            color_view,
            depth_texture,
            depth_view,
            vehicle_pipeline,
            ui_pipeline,
            format,
        }
    }

    /// Render a single frame and read back raw RGBA8 pixels.
    pub async fn render_to_rgba8(
        &mut self,
        context: &GpuContext,
        mesh: &GpuMesh,
        texture_mgr: &TextureManager,
        camera: &OrbitCamera,
        controls: Option<&TouchManager>,
    ) -> Vec<u8> {
        let uniforms = camera.build_uniforms();
        self.vehicle_pipeline
            .camera_uniforms
            .update(&context.queue, &uniforms);

        if let Some(_ctrl) = controls {
            self.ui_pipeline.update_screen_size(
                &context.queue,
                self.width as f32,
                self.height as f32,
            );
        }

        let mut encoder = context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Offscreen Render Encoder"),
            });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Offscreen Vehicle Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.08,
                            g: 0.08,
                            b: 0.12,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // 1. Draw 3D vehicle
            self.vehicle_pipeline
                .render_mesh(&mut rpass, mesh, texture_mgr);

            // 2. Draw 2D virtual controls overlay if requested
            if let Some(ctrl) = controls {
                self.ui_pipeline
                    .render_controls(&context.queue, &mut rpass, ctrl);
            }
        }

        // Align bytes per row to 256 for wgpu buffer copy requirements
        let bytes_per_pixel = 4u32;
        let unaligned_bytes_per_row = self.width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = unaligned_bytes_per_row.div_ceil(align) * align;
        let buffer_size = (padded_bytes_per_row * self.height) as u64;

        let staging_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Offscreen Staging Read Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.color_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &staging_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        context.queue.submit(Some(encoder.finish()));

        // Map buffer and read back pixels
        let buffer_slice = staging_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        context.device.poll(wgpu::Maintain::Wait);
        rx.recv().unwrap().unwrap();

        let padded_data = buffer_slice.get_mapped_range();
        let mut unpadded_rgba = Vec::with_capacity((self.width * self.height * 4) as usize);

        for row in 0..self.height {
            let start = (row * padded_bytes_per_row) as usize;
            let end = start + unaligned_bytes_per_row as usize;
            unpadded_rgba.extend_from_slice(&padded_data[start..end]);
        }

        drop(padded_data);
        staging_buffer.unmap();

        unpadded_rgba
    }

    /// Save RGBA8 buffer as standard PPM (Portable Pixmap) image.
    pub fn save_ppm(
        path: &Path,
        width: u32,
        height: u32,
        rgba_pixels: &[u8],
    ) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        writeln!(file, "P6\n{} {}\n255", width, height)?;
        let mut rgb = Vec::with_capacity((width * height * 3) as usize);
        for chunk in rgba_pixels.chunks_exact(4) {
            rgb.push(chunk[0]); // R
            rgb.push(chunk[1]); // G
            rgb.push(chunk[2]); // B
        }
        file.write_all(&rgb)?;
        Ok(())
    }
}
