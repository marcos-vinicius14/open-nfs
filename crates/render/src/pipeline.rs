//! Render pipeline initialization, vertex buffer layout, and depth-stencil state.
//!
//! Conforms to REQ-M3-01, REQ-M3-02, REQ-M3-03, and Section 1 of M3 guidelines.

use crate::camera::CameraUniforms;
use crate::controls::TouchManager;
use crate::mesh::GpuMesh;
use crate::texture::{GpuTexture, TextureManager};
use bytemuck::{Pod, Zeroable};
use open_nfs_asset_runtime::model::RuntimeVertex;
use wgpu::util::DeviceExt;

/// Uniform buffer for camera projection, view matrix, and lighting.
#[derive(Debug)]
pub struct CameraUniformBuffer {
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub layout: wgpu::BindGroupLayout,
}

impl CameraUniformBuffer {
    pub fn new(device: &wgpu::Device) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Camera Uniforms Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let initial_uniforms = CameraUniforms::default();
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Uniform Buffer"),
            contents: bytemuck::bytes_of(&initial_uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Uniform Bind Group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        Self {
            buffer,
            bind_group,
            layout,
        }
    }

    pub fn update(&self, queue: &wgpu::Queue, uniforms: &CameraUniforms) {
        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(uniforms));
    }
}

/// Pipeline for rendering 3D vehicle geometry with textures, vertex color, and lighting.
#[derive(Debug)]
pub struct VehiclePipeline {
    pub pipeline: wgpu::RenderPipeline,
    pub camera_uniforms: CameraUniformBuffer,
    pub texture_layout: wgpu::BindGroupLayout,
}

impl VehiclePipeline {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
    ) -> Self {
        let camera_uniforms = CameraUniformBuffer::new(device);
        let texture_layout = GpuTexture::create_bind_group_layout(device);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Vehicle WGSL Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Vehicle Pipeline Layout"),
            bind_group_layouts: &[&camera_uniforms.layout, &texture_layout],
            push_constant_ranges: &[],
        });

        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<RuntimeVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position: [f32; 3]
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Normal: [f32; 3]
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Color: u32 -> Unorm8x4
                wgpu::VertexAttribute {
                    offset: 24,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Unorm8x4,
                },
                // UV: [f32; 2]
                wgpu::VertexAttribute {
                    offset: 28,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        };

        let depth_stencil = depth_format.map(|format| wgpu::DepthStencilState {
            format,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Vehicle Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[vertex_buffer_layout],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // Render both sides to support interior, badges, and spoilers
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            camera_uniforms,
            texture_layout,
        }
    }

    /// Record draw commands for a vehicle mesh using its submeshes and texture manager.
    pub fn render_mesh<'a>(
        &'a self,
        rpass: &mut wgpu::RenderPass<'a>,
        mesh: &'a GpuMesh,
        texture_mgr: &'a TextureManager,
    ) {
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &self.camera_uniforms.bind_group, &[]);
        mesh.bind(rpass);

        if mesh.submeshes.is_empty() {
            // Draw whole mesh with fallback texture
            rpass.set_bind_group(1, &texture_mgr.get_fallback().bind_group, &[]);
            mesh.draw_all(rpass);
        } else {
            for (idx, submesh) in mesh.submeshes.iter().enumerate() {
                let tex = texture_mgr.get_or_fallback(submesh.material_id);
                rpass.set_bind_group(1, &tex.bind_group, &[]);
                mesh.draw_submesh(rpass, idx);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 2D UI Overlay Pipeline for Virtual Controls
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct ScreenUniforms {
    screen_size: [f32; 2],
    padding: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct UiVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
}

/// Pipeline for rendering 2D touch control overlays (virtual steering wheel and pedals).
#[derive(Debug)]
pub struct UiPipeline {
    pub pipeline: wgpu::RenderPipeline,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub max_vertices: usize,
    pub max_indices: usize,
}

impl UiPipeline {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
    ) -> Self {
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("UI Screen Uniform Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let initial_screen = ScreenUniforms {
            screen_size: [1280.0, 720.0],
            padding: [0.0, 0.0],
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("UI Screen Uniform Buffer"),
            contents: bytemuck::bytes_of(&initial_screen),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("UI Screen Uniform Bind Group"),
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("UI WGSL Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("ui_shader.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("UI Pipeline Layout"),
            bind_group_layouts: &[&uniform_layout],
            push_constant_ranges: &[],
        });

        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<UiVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 8,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        };

        let depth_stencil = depth_format.map(|format| wgpu::DepthStencilState {
            format,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::Always,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("UI Overlay Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_ui"),
                buffers: &[vertex_layout],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_ui"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let max_vertices = 512;
        let max_indices = 1024;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("UI Dynamic Vertex Buffer"),
            size: (max_vertices * std::mem::size_of::<UiVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("UI Dynamic Index Buffer"),
            size: (max_indices * std::mem::size_of::<u16>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            uniform_buffer,
            uniform_bind_group,
            vertex_buffer,
            index_buffer,
            max_vertices,
            max_indices,
        }
    }

    /// Update screen resolution uniform.
    pub fn update_screen_size(&self, queue: &wgpu::Queue, width: f32, height: f32) {
        let uniforms = ScreenUniforms {
            screen_size: [width, height],
            padding: [0.0, 0.0],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
    }

    /// Render virtual controls (steering wheel rim & indicator, throttle & brake pedals).
    pub fn render_controls<'a>(
        &'a self,
        queue: &wgpu::Queue,
        rpass: &mut wgpu::RenderPass<'a>,
        controls: &TouchManager,
    ) {
        let mut vertices: Vec<UiVertex> = Vec::with_capacity(256);
        let mut indices: Vec<u16> = Vec::with_capacity(512);

        // 1. Draw steering wheel rim (ring)
        let sw = &controls.steering;
        let rim_segments = 24;
        let center_x = sw.center_x;
        let center_y = sw.center_y;
        let r_outer = sw.radius;
        let r_inner = sw.radius * 0.82;

        let wheel_color = if sw.is_active {
            [0.0, 0.85, 1.0, 0.80] // Bright cyan when active
        } else {
            [0.9, 0.9, 0.95, 0.55] // Clean translucent white
        };

        let start_v = vertices.len() as u16;
        for i in 0..rim_segments {
            let angle = (i as f32 / rim_segments as f32) * std::f32::consts::TAU;
            let cos_a = angle.cos();
            let sin_a = angle.sin();
            vertices.push(UiVertex {
                position: [center_x + r_outer * cos_a, center_y + r_outer * sin_a],
                color: wheel_color,
            });
            vertices.push(UiVertex {
                position: [center_x + r_inner * cos_a, center_y + r_inner * sin_a],
                color: wheel_color,
            });
        }
        for i in 0..rim_segments {
            let next = (i + 1) % rim_segments;
            let i0 = start_v + (i as u16) * 2;
            let i1 = i0 + 1;
            let i2 = start_v + (next as u16) * 2;
            let i3 = i2 + 1;

            indices.extend_from_slice(&[i0, i1, i2, i2, i1, i3]);
        }

        // Steering hub
        let hub_segments = 16;
        let hub_r = sw.radius * 0.28;
        let hub_v = vertices.len() as u16;
        let hub_col = if sw.is_active {
            [0.0, 0.7, 0.9, 0.75]
        } else {
            [0.7, 0.7, 0.75, 0.45]
        };
        vertices.push(UiVertex {
            position: [center_x, center_y],
            color: hub_col,
        });
        for i in 0..hub_segments {
            let angle = (i as f32 / hub_segments as f32) * std::f32::consts::TAU;
            vertices.push(UiVertex {
                position: [
                    center_x + hub_r * angle.cos(),
                    center_y + hub_r * angle.sin(),
                ],
                color: hub_col,
            });
        }
        for i in 0..hub_segments {
            let next = (i + 1) % hub_segments;
            indices.extend_from_slice(&[hub_v, hub_v + 1 + (i as u16), hub_v + 1 + (next as u16)]);
        }

        // 2. Draw steering direction indicator spoke
        let spoke_angle = sw.current_angle_rad - std::f32::consts::FRAC_PI_2;
        let spoke_w = 8.0_f32;
        let cos_s = spoke_angle.cos();
        let sin_s = spoke_angle.sin();
        let perp_x = -sin_s * spoke_w;
        let perp_y = cos_s * spoke_w;

        let spoke_color = [1.0, 0.85, 0.1, 0.95]; // Bright amber indicator
        let spoke_start = vertices.len() as u16;
        vertices.push(UiVertex {
            position: [center_x + perp_x, center_y + perp_y],
            color: spoke_color,
        });
        vertices.push(UiVertex {
            position: [center_x - perp_x, center_y - perp_y],
            color: spoke_color,
        });
        vertices.push(UiVertex {
            position: [
                center_x + cos_s * r_outer + perp_x,
                center_y + sin_s * r_outer + perp_y,
            ],
            color: spoke_color,
        });
        vertices.push(UiVertex {
            position: [
                center_x + cos_s * r_outer - perp_x,
                center_y + sin_s * r_outer - perp_y,
            ],
            color: spoke_color,
        });
        indices.extend_from_slice(&[
            spoke_start,
            spoke_start + 1,
            spoke_start + 2,
            spoke_start + 2,
            spoke_start + 1,
            spoke_start + 3,
        ]);

        // 3. Draw Brake Pedal (left of throttle)
        let bp = &controls.pedals;
        let brake_active = bp.brake_pointer_id.is_some() || bp.brake_value > 0.05;
        let brake_color = if brake_active {
            [0.95, 0.15, 0.15, 0.85] // Red active
        } else {
            [0.75, 0.15, 0.15, 0.55]
        };
        push_rect(
            &mut vertices,
            &mut indices,
            bp.brake_x,
            bp.brake_y,
            bp.brake_w,
            bp.brake_h,
            brake_color,
        );
        if bp.brake_value > 0.02 {
            let fill_h = (bp.brake_h - 8.0) * bp.brake_value;
            push_rect(
                &mut vertices,
                &mut indices,
                bp.brake_x + 4.0,
                bp.brake_y + bp.brake_h - 4.0 - fill_h,
                bp.brake_w - 8.0,
                fill_h,
                [1.0, 0.4, 0.4, 0.95],
            );
        }

        // 4. Draw Throttle Pedal (right pedal)
        let throttle_active = bp.throttle_pointer_id.is_some() || bp.throttle_value > 0.05;
        let throttle_color = if throttle_active {
            [0.2, 0.95, 0.35, 0.85] // Green active
        } else {
            [0.15, 0.75, 0.25, 0.55]
        };
        push_rect(
            &mut vertices,
            &mut indices,
            bp.throttle_x,
            bp.throttle_y,
            bp.throttle_w,
            bp.throttle_h,
            throttle_color,
        );
        if bp.throttle_value > 0.02 {
            let fill_h = (bp.throttle_h - 8.0) * bp.throttle_value;
            push_rect(
                &mut vertices,
                &mut indices,
                bp.throttle_x + 4.0,
                bp.throttle_y + bp.throttle_h - 4.0 - fill_h,
                bp.throttle_w - 8.0,
                fill_h,
                [0.4, 1.0, 0.5, 0.95],
            );
        }

        if vertices.len() <= self.max_vertices && indices.len() <= self.max_indices {
            queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
            queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&indices));

            rpass.set_pipeline(&self.pipeline);
            rpass.set_bind_group(0, &self.uniform_bind_group, &[]);
            rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            rpass.draw_indexed(0..(indices.len() as u32), 0, 0..1);
        }
    }
}

fn push_rect(
    vertices: &mut Vec<UiVertex>,
    indices: &mut Vec<u16>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: [f32; 4],
) {
    let start = vertices.len() as u16;
    vertices.push(UiVertex {
        position: [x, y],
        color,
    });
    vertices.push(UiVertex {
        position: [x + w, y],
        color,
    });
    vertices.push(UiVertex {
        position: [x, y + h],
        color,
    });
    vertices.push(UiVertex {
        position: [x + w, y + h],
        color,
    });

    indices.extend_from_slice(&[start, start + 2, start + 1, start + 1, start + 2, start + 3]);
}
