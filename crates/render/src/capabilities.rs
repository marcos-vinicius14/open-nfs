//! GPU capability probe and hardware diagnostics.
//!
//! Conforms to M3-02, REQ-M3-01, and EV-002: probes Vulkan/wgpu adapter features,
//! limits, and backend support on physical target devices and host desktop.

use wgpu::{Adapter, AdapterInfo, Limits};

/// Structured report of detected GPU capabilities and hardware limits.
#[derive(Debug, Clone, PartialEq)]
pub struct GpuCapabilitiesReport {
    pub device_name: String,
    pub vendor_id: u32,
    pub device_id: u32,
    pub device_type: String,
    pub driver_info: String,
    pub backend: String,
    pub max_texture_dimension_2d: u32,
    pub max_buffer_size: u64,
    pub max_bind_groups: u32,
    pub max_vertex_buffers: u32,
    pub supports_bc_compression: bool,
    pub supports_etc2_compression: bool,
    pub supports_astc_compression: bool,
    pub supports_timestamp_queries: bool,
}

impl GpuCapabilitiesReport {
    /// Probe the adapter capabilities, features, and hardware limits.
    pub fn probe(adapter: &Adapter) -> Self {
        let info: AdapterInfo = adapter.get_info();
        let limits: Limits = adapter.limits();
        let features = adapter.features();

        Self {
            device_name: info.name,
            vendor_id: info.vendor,
            device_id: info.device,
            device_type: format!("{:?}", info.device_type),
            driver_info: info.driver_info,
            backend: format!("{:?}", info.backend),
            max_texture_dimension_2d: limits.max_texture_dimension_2d,
            max_buffer_size: limits.max_buffer_size,
            max_bind_groups: limits.max_bind_groups,
            max_vertex_buffers: limits.max_vertex_buffers,
            supports_bc_compression: features.contains(wgpu::Features::TEXTURE_COMPRESSION_BC),
            supports_etc2_compression: features.contains(wgpu::Features::TEXTURE_COMPRESSION_ETC2),
            supports_astc_compression: features.contains(wgpu::Features::TEXTURE_COMPRESSION_ASTC),
            supports_timestamp_queries: features.contains(wgpu::Features::TIMESTAMP_QUERY),
        }
    }

    /// Format the capability report as Markdown for inclusion in benchmarks.
    pub fn to_markdown(&self) -> String {
        format!(
            "### GPU Hardware & Capability Baseline\n\n\
             - **Device Name:** `{}`\n\
             - **Vendor ID:** `0x{:04X}` | **Device ID:** `0x{:04X}`\n\
             - **Device Type:** `{}`\n\
             - **Backend:** `{}`\n\
             - **Driver Info:** `{}`\n\
             - **Max 2D Texture Size:** `{} x {}` px\n\
             - **Max GPU Buffer Size:** `{:.2} MiB`\n\
             - **Max Bind Groups:** `{}`\n\
             - **Compressed Textures Supported:** BC: `{}` | ETC2: `{}` | ASTC: `{}`\n\
             - **Timestamp Profiling:** `{}`\n",
            self.device_name,
            self.vendor_id,
            self.device_id,
            self.device_type,
            self.backend,
            self.driver_info,
            self.max_texture_dimension_2d,
            self.max_texture_dimension_2d,
            self.max_buffer_size as f64 / (1024.0 * 1024.0),
            self.max_bind_groups,
            self.supports_bc_compression,
            self.supports_etc2_compression,
            self.supports_astc_compression,
            self.supports_timestamp_queries,
        )
    }
}
