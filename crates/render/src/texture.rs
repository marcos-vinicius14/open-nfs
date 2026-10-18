//! GPU texture management, mipmaps, and bind group creation.
//!
//! Conforms to REQ-M3-05 and Section 2 of M3 guidelines:
//! GPU-resident textures, shared sampler reuse, fallback checkerboard/white textures.

use std::collections::HashMap;

/// GPU texture wrapping a wgpu Texture, View, Sampler, and BindGroup.
#[derive(Debug)]
pub struct GpuTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub bind_group: wgpu::BindGroup,
    pub width: u32,
    pub height: u32,
}

impl GpuTexture {
    /// Create the canonical bind group layout for diffuse textures (Binding 0 = Texture, Binding 1 = Sampler).
    pub fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Diffuse Texture Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    }

    /// Upload an RGBA8 byte buffer to a new GPU texture and construct its bind group.
    pub fn from_rgba8(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        width: u32,
        height: u32,
        rgba_data: &[u8],
        label: Option<&str>,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width.max(1)),
                rows_per_image: Some(height.max(1)),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Diffuse Texture Linear Sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Diffuse Texture Bind Group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            texture,
            view,
            sampler,
            bind_group,
            width: width.max(1),
            height: height.max(1),
        }
    }

    /// Create a 1x1 solid white fallback texture.
    pub fn create_fallback_white(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
    ) -> Self {
        Self::create_solid_color(
            device,
            queue,
            layout,
            [255, 255, 255, 255],
            Some("Fallback White Texture"),
        )
    }

    /// Create a 4x4 solid RGBA8 material texture.
    pub fn create_solid_color(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        rgba: [u8; 4],
        label: Option<&str>,
    ) -> Self {
        let mut pixels = [0u8; 4 * 4 * 4];
        for chunk in pixels.chunks_exact_mut(4) {
            chunk.copy_from_slice(&rgba);
        }
        Self::from_rgba8(device, queue, layout, 4, 4, &pixels, label)
    }

    /// Create a 32x32 magenta and black checkerboard texture for missing textures.
    pub fn create_checkerboard(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let dim = 32usize;
        let mut data = Vec::with_capacity(dim * dim * 4);
        for y in 0..dim {
            for x in 0..dim {
                let is_magenta = ((x / 4) + (y / 4)) % 2 == 0;
                if is_magenta {
                    data.extend_from_slice(&[255, 0, 255, 255]); // Magenta
                } else {
                    data.extend_from_slice(&[0, 0, 0, 255]); // Black
                }
            }
        }
        Self::from_rgba8(
            device,
            queue,
            layout,
            dim as u32,
            dim as u32,
            &data,
            Some("Checkerboard Fallback Texture"),
        )
    }
}

/// Texture cache and manager mapping texture hashes to GPU textures.
#[derive(Debug)]
pub struct TextureManager {
    pub textures: HashMap<u32, GpuTexture>,
    pub fallback: GpuTexture,
}

impl TextureManager {
    pub fn new(fallback: GpuTexture) -> Self {
        Self {
            textures: HashMap::new(),
            fallback,
        }
    }

    /// Populate standard NFSU2 vehicle material and shader textures (`CARSKIN`, `WINDSHIELD`, `CHROME`,
    /// `RUBBER`, `BRAKELIGHT`, `HEADLIGHT`, `CALIPER`, `BRAKEDISC`, `DULLPLASTIC`, etc.) if not already loaded.
    pub fn register_nfsu2_materials(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
    ) {
        let entries: &[(u32, [u8; 4], &str)] = &[
            // Car Body Paint (CARSKIN / 350Z_SKIN1) -> Metallic Daytona Blue
            (0xD6D6_080A, [35, 115, 220, 255], "Mat CARSKIN"),
            (0x3C84_D757, [35, 115, 220, 255], "Mat CARSKIN_TEX"),
            (0xA675_4656, [35, 115, 220, 255], "Mat 350Z_SKIN1"),
            (0xA736_6AE6, [35, 115, 220, 255], "Mat SKIRT_SKIN"),
            // Windows & Windshield -> Dark Smoked Glass
            (0x471A_1DCA, [28, 38, 50, 255], "Mat WINDSHIELD"),
            (0x7B22_0DDF, [28, 38, 50, 255], "Mat WINDOW_FRONT"),
            (0x4CDE_BFCA, [28, 38, 50, 255], "Mat WINDOW_REAR_BASE"),
            (0xE7E4_EF49, [28, 38, 50, 255], "Mat WINDOW_LEFT"),
            (0x60F8_B13C, [28, 38, 50, 255], "Mat WINDOW_RIGHT"),
            (0xD195_BE56, [28, 38, 50, 255], "Mat WINDOW_TRUNK"),
            (0x3ED7_0C43, [22, 30, 40, 255], "Mat WINDOW_MASK"),
            // Chrome, Aluminum, Mag Silver -> Bright Metallic Silver
            (0x5494_9AFD, [220, 226, 235, 255], "Mat CHROME"),
            (0x2E65_E067, [195, 202, 212, 255], "Mat ALUMINUM"),
            (0x2271_9FA9, [205, 212, 220, 255], "Mat MAGSILVER"),
            // Headlight Lens & Reflector -> Cool Xenon White
            (0x9C64_5529, [230, 240, 255, 255], "Mat HEADLIGHT"),
            (0xA7E6_EA53, [230, 240, 255, 255], "Mat HEADLIGHT_LEFT"),
            (0xA532_FC46, [230, 240, 255, 255], "Mat HEADLIGHT_RIGHT"),
            (0xA634_8EE3, [0, 0, 0, 0], "Mat HEADLIGHT_GLASS"),
            (0x267D_188C, [0, 0, 0, 0], "Mat HEADLIGHT_LENS_L"),
            (0xF68E_F19F, [0, 0, 0, 0], "Mat HEADLIGHT_LENS_R"),
            // Brakelights / Taillights -> Ruby Red Lens
            (0x05BC_3A3C, [215, 25, 35, 255], "Mat BRAKELIGHT"),
            (0xD795_97D6, [215, 25, 35, 255], "Mat BRAKELIGHT_GLASS"),
            (0xF7FC_7674, [220, 30, 40, 255], "Mat LIGHT_LENS_RED"),
            (0xD947_F346, [215, 25, 35, 255], "Mat BRAKELIGHT_LEFT"),
            (0x02B5_2399, [215, 25, 35, 255], "Mat BRAKELIGHT_RIGHT"),
            (0x000B_CBBF, [215, 25, 35, 255], "Mat BRAKELIGHT_SUB1"),
            (0x01F4_0B32, [215, 25, 35, 255], "Mat BRAKELIGHT_SUB2"),
            (0xEAD8_676E, [215, 25, 35, 255], "Mat 350Z_BRAKELIGHT_OFF"),
            (0x3617_DCFC, [215, 25, 35, 255], "Mat TRUNK_BRAKELIGHT"),
            // Tire Rubber, Dull Plastic, Grille Mesh, Interior & Chassis Trim -> Dark Charcoal
            (0x7874_3CA1, [38, 40, 44, 255], "Mat RUBBER"),
            (0x0FED_EE40, [45, 48, 54, 255], "Mat DULLPLASTIC"),
            (0x010C_B64A, [40, 43, 48, 255], "Mat INTERIOR_TRIM"),
            (0x12C9_453C, [32, 34, 38, 255], "Mat BLACK_TRIM"),
            (0x33A2_6CB6, [36, 38, 42, 255], "Mat GRILLE_MESH"),
            (0x52BF_4C34, [30, 32, 36, 255], "Mat BOTTOM"),
            (0xBD1F_0E0A, [34, 36, 40, 255], "Mat BUMPER_GRILLE"),
            (0x5799_E60B, [48, 50, 56, 255], "Mat DRIVER"),
            (0x2787_EDAB, [52, 55, 62, 255], "Mat CHASSIS_METAL"),
            // Brake Discs, Calipers & Engine Bay
            (0xF0A4_3B67, [160, 166, 175, 255], "Mat BRAKEDISC"),
            (0xAF09_F56C, [150, 156, 165, 255], "Mat ROTOR_DISC"),
            (0xC657_8694, [150, 156, 165, 255], "Mat ROTOR_TEX"),
            (0xD664_0DFF, [215, 155, 30, 255], "Mat CALIPER_GOLD"),
            (0x621F_43FC, [215, 155, 30, 255], "Mat 350Z_MISC"),
            (0x54AA_F7C6, [95, 100, 108, 255], "Mat 350Z_ENGINE"),
            (0x1204_9251, [110, 115, 125, 255], "Mat ENGINE_METAL"),
        ];

        for &(hash, rgba, label) in entries {
            self.textures.entry(hash).or_insert_with(|| {
                GpuTexture::create_solid_color(device, queue, layout, rgba, Some(label))
            });
        }
    }

    pub fn insert(&mut self, hash: u32, texture: GpuTexture) {
        self.textures.insert(hash, texture);
    }

    pub fn get_or_fallback(&self, hash: u32) -> &GpuTexture {
        self.textures.get(&hash).unwrap_or(&self.fallback)
    }

    pub fn get_fallback(&self) -> &GpuTexture {
        &self.fallback
    }

    pub fn len(&self) -> usize {
        self.textures.len()
    }

    pub fn is_empty(&self) -> bool {
        self.textures.is_empty()
    }
}
