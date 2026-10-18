//! Surface lifecycle, swapchain management, and resize handling.
//!
//! Conforms to REQ-M3-01, REQ-M3-11, and Section 6 of M3 guidelines:
//! Resilient to window minimize, pause/resume, and zero-dimension viewports.

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// Surface manager wrapping wgpu::Surface, depth texture, and swapchain configuration.
pub struct SurfaceManager<'w> {
    pub surface: wgpu::Surface<'w>,
    pub config: wgpu::SurfaceConfiguration,
    pub depth_texture: wgpu::Texture,
    pub depth_view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
    pub is_ready: bool,
}

impl<'w> SurfaceManager<'w> {
    /// Initialize a new SurfaceManager and configure the swapchain.
    pub fn new(
        surface: wgpu::Surface<'w>,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> Self {
        let caps = surface.get_capabilities(adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);

        let present_mode = if caps.present_modes.contains(&wgpu::PresentMode::Fifo) {
            wgpu::PresentMode::Fifo // VSync on mobile
        } else {
            caps.present_modes[0]
        };

        let width = width.max(1);
        let height = height.max(1);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(device, &config);

        let (depth_texture, depth_view) = Self::create_depth_texture(device, width, height);

        Self {
            surface,
            config,
            depth_texture,
            depth_view,
            width,
            height,
            is_ready: true,
        }
    }

    /// Resize the surface swapchain and depth buffer.
    /// Safely handles zero-dimension minimize/background events on Android.
    pub fn resize(&mut self, device: &wgpu::Device, new_width: u32, new_height: u32) {
        if new_width == 0 || new_height == 0 {
            self.is_ready = false;
            return;
        }

        self.width = new_width;
        self.height = new_height;
        self.config.width = new_width;
        self.config.height = new_height;

        self.surface.configure(device, &self.config);

        let (depth_tex, depth_view) = Self::create_depth_texture(device, new_width, new_height);
        self.depth_texture = depth_tex;
        self.depth_view = depth_view;
        self.is_ready = true;
    }

    /// Acquire the next surface texture for rendering.
    pub fn acquire(&self) -> Result<wgpu::SurfaceTexture, wgpu::SurfaceError> {
        self.surface.get_current_texture()
    }

    /// Reconfigure the surface if swapchain becomes outdated or lost.
    pub fn reconfigure(&mut self, device: &wgpu::Device) {
        if self.width > 0 && self.height > 0 {
            self.surface.configure(device, &self.config);
        }
    }

    fn create_depth_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let size = wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        };

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        (depth_texture, depth_view)
    }
}
