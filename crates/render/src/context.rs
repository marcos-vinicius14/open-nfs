//! GPU context managing Instance, Adapter, Device, and Queue initialization.

use crate::capabilities::GpuCapabilitiesReport;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("Failed to find a compatible graphics adapter with requested backends")]
    NoAdapterFound,

    #[error("Failed to request GPU device: {0}")]
    DeviceRequestFailed(#[from] wgpu::RequestDeviceError),

    #[error("Surface creation or acquisition error: {0}")]
    SurfaceError(#[from] wgpu::SurfaceError),

    #[error("Texture loading or format error: {0}")]
    TextureError(String),
}

/// Persistent GPU context holding the device, queue, and probed adapter capabilities.
#[derive(Debug)]
pub struct GpuContext {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub capabilities: GpuCapabilitiesReport,
}

impl GpuContext {
    /// Initialize a headless GPU context for offscreen rendering, capability probing, or testing.
    pub async fn new_headless() -> Result<Self, RenderError> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN | wgpu::Backends::GL,
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or(RenderError::NoAdapterFound)?;

        Self::from_adapter(instance, adapter).await
    }

    /// Initialize a GPU context compatible with the given target surface.
    pub async fn new_with_surface(
        instance: wgpu::Instance,
        surface: &wgpu::Surface<'_>,
    ) -> Result<Self, RenderError> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or(RenderError::NoAdapterFound)?;

        Self::from_adapter(instance, adapter).await
    }

    async fn from_adapter(
        instance: wgpu::Instance,
        adapter: wgpu::Adapter,
    ) -> Result<Self, RenderError> {
        let capabilities = GpuCapabilitiesReport::probe(&adapter);
        tracing::info!(
            device = %capabilities.device_name,
            backend = %capabilities.backend,
            "Initializing GPU Context with Vulkan/wgpu"
        );

        let required_limits =
            wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits());
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Open-NFS Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits,
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await?;

        Ok(Self {
            instance,
            adapter,
            device: Arc::new(device),
            queue: Arc::new(queue),
            capabilities,
        })
    }
}
