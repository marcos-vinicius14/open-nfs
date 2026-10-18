//! `open-nfs-android-host` — Android host integration and platform isolation layer.
//!
//! Conforms to ADR-002 and NFR-006 by isolating Android platform specifics behind
//! platform-agnostic lifecycle and input traits.

#![deny(unsafe_code)]

use open_nfs_core::types::FixedDelta;

/// Surface lifecycle state for graphics pipeline recreation and presentation pacing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceState {
    /// No native window/surface is currently attached.
    Detached,
    /// Native surface is active with positive dimensions.
    Active { width: u32, height: u32 },
    /// Surface is paused or suspended (app in background).
    Suspended,
}

/// Normalized touch input event originating from mobile touchscreen or desktop simulation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TouchEvent {
    Down { id: u64, x: f32, y: f32 },
    Move { id: u64, x: f32, y: f32 },
    Up { id: u64, x: f32, y: f32 },
    Cancel { id: u64 },
}

/// Platform-agnostic engine host contract.
///
/// Implementations handle native platform dispatch without exposing OS-specific
/// types to the simulation or rendering crates.
pub trait PlatformHost {
    /// Query the current surface state.
    fn surface_state(&self) -> SurfaceState;

    /// Process a lifecycle event (e.g. pause/resume).
    fn on_lifecycle_change(&mut self, state: SurfaceState);

    /// Dispatch incoming touch interaction.
    fn on_touch(&mut self, event: TouchEvent);

    /// Step simulation or presentation by fixed delta.
    fn tick(&mut self, delta: FixedDelta);
}

/// Baseline mock host implementation used for unit testing and desktop smoke runs.
#[derive(Debug, Default)]
pub struct MockHost {
    pub surface: Option<SurfaceState>,
    pub last_touch: Option<TouchEvent>,
    pub tick_count: u64,
}

impl MockHost {
    pub fn new() -> Self {
        Self {
            surface: Some(SurfaceState::Detached),
            last_touch: None,
            tick_count: 0,
        }
    }
}

impl PlatformHost for MockHost {
    fn surface_state(&self) -> SurfaceState {
        self.surface.unwrap_or(SurfaceState::Detached)
    }

    fn on_lifecycle_change(&mut self, state: SurfaceState) {
        tracing::debug!(?state, "Host surface state changed");
        self.surface = Some(state);
    }

    fn on_touch(&mut self, event: TouchEvent) {
        self.last_touch = Some(event);
    }

    fn tick(&mut self, _delta: FixedDelta) {
        self.tick_count += 1;
    }
}

use open_nfs_asset_runtime::model::{
    RuntimeBounds, RuntimeModel, RuntimePart, RuntimeSubmesh, RuntimeVertex,
};

pub fn generate_procedural_car() -> RuntimeModel {
    let mut parts = Vec::new();

    let body_verts = vec![
        RuntimeVertex {
            position: [1.8, -0.9, 0.4],
            normal: [0.7, 0.0, 0.7],
            color: 0xFF2020D0,
            uv: [0.0, 0.0],
        },
        RuntimeVertex {
            position: [1.8, 0.9, 0.4],
            normal: [0.7, 0.0, 0.7],
            color: 0xFF2020D0,
            uv: [1.0, 0.0],
        },
        RuntimeVertex {
            position: [0.5, 0.8, 0.7],
            normal: [0.3, 0.0, 0.9],
            color: 0xFF2020D0,
            uv: [1.0, 0.5],
        },
        RuntimeVertex {
            position: [0.5, -0.8, 0.7],
            normal: [0.3, 0.0, 0.9],
            color: 0xFF2020D0,
            uv: [0.0, 0.5],
        },
        RuntimeVertex {
            position: [-0.4, -0.7, 1.2],
            normal: [0.0, 0.0, 1.0],
            color: 0xFF2020D0,
            uv: [0.1, 0.8],
        },
        RuntimeVertex {
            position: [-0.4, 0.7, 1.2],
            normal: [0.0, 0.0, 1.0],
            color: 0xFF2020D0,
            uv: [0.9, 0.8],
        },
        RuntimeVertex {
            position: [-1.9, 0.85, 0.6],
            normal: [-0.8, 0.0, 0.5],
            color: 0xFF2020D0,
            uv: [1.0, 1.0],
        },
        RuntimeVertex {
            position: [-1.9, -0.85, 0.6],
            normal: [-0.8, 0.0, 0.5],
            color: 0xFF2020D0,
            uv: [0.0, 1.0],
        },
    ];
    let body_indices = vec![
        0, 1, 2, 0, 2, 3, // Hood
        3, 2, 5, 3, 5, 4, // Windshield
        4, 5, 6, 4, 6, 7, // Roof & rear deck
    ];

    parts.push(RuntimePart {
        name: "KIT00_BODY_A".to_string(),
        lod_level: 0,
        part_hash: 0x1111,
        bounds: RuntimeBounds::default(),
        vertices: body_verts,
        indices: body_indices,
        submeshes: vec![RuntimeSubmesh {
            material_id: 0,
            index_offset: 0,
            index_count: 18,
            shader_flags: 0,
            bounds: RuntimeBounds::default(),
        }],
    });

    RuntimeModel {
        source_sha256: [0u8; 32],
        parts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_lifecycle_transitions() {
        let mut host = MockHost::new();
        assert_eq!(host.surface_state(), SurfaceState::Detached);

        host.on_lifecycle_change(SurfaceState::Active {
            width: 1920,
            height: 1080,
        });
        assert_eq!(
            host.surface_state(),
            SurfaceState::Active {
                width: 1920,
                height: 1080
            }
        );

        host.on_lifecycle_change(SurfaceState::Suspended);
        assert_eq!(host.surface_state(), SurfaceState::Suspended);
    }

    #[test]
    fn touch_event_dispatch() {
        let mut host = MockHost::new();
        let touch = TouchEvent::Down {
            id: 1,
            x: 100.0,
            y: 200.0,
        };
        host.on_touch(touch);
        assert_eq!(host.last_touch, Some(touch));
    }

    #[test]
    fn fixed_step_tick() {
        let mut host = MockHost::new();
        host.tick(FixedDelta::STEP_60HZ);
        host.tick(FixedDelta::STEP_60HZ);
        assert_eq!(host.tick_count, 2);
    }
}

#[cfg(target_os = "android")]
// SAFETY: Encapsulates unsafe raw window handle conversion for Android NDK native window.
#[allow(unsafe_code)]
fn create_android_surface<'a>(
    instance: &wgpu::Instance,
    native_window: &ndk::native_window::NativeWindow,
) -> Result<wgpu::Surface<'a>, wgpu::CreateSurfaceError> {
    use raw_window_handle::{AndroidDisplayHandle, HasWindowHandle, RawDisplayHandle};

    let raw_display_handle = RawDisplayHandle::Android(AndroidDisplayHandle::new());
    let raw_window_handle = native_window.window_handle().unwrap().as_raw();

    // SAFETY: The NativeWindow pointer is acquired from the AndroidApp lifecycle and remains
    // valid until TerminateWindow is dispatched by the OS.
    unsafe {
        instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
            raw_display_handle,
            raw_window_handle,
        })
    }
}

#[cfg(target_os = "android")]
// SAFETY: FFI entry point required by Android NDK NativeActivity runtime.
#[cfg(target_os = "android")]
#[allow(unsafe_code)]
pub fn log_android(msg: &str) {
    use android_activity::ndk_sys;
    use std::ffi::CString;
    if let Ok(tag) = CString::new("OpenNFS") {
        if let Ok(text) = CString::new(msg) {
            unsafe {
                ndk_sys::__android_log_write(
                    ndk_sys::android_LogPriority::ANDROID_LOG_INFO.0 as i32,
                    tag.as_ptr(),
                    text.as_ptr(),
                );
            }
        }
    }
}

#[cfg(target_os = "android")]
// SAFETY: FFI entry point required by Android NDK NativeActivity runtime.
#[allow(unsafe_code)]
#[no_mangle]
fn android_main(app: android_activity::AndroidApp) {
    use android_activity::input::{InputEvent, MotionAction};
    use android_activity::{InputStatus, MainEvent, PollEvent};
    use open_nfs_render::camera::OrbitCamera;
    use open_nfs_render::context::GpuContext;
    use open_nfs_render::controls::TouchManager;
    use open_nfs_render::diagnostics::RenderDiagnostics;
    use open_nfs_render::mesh::GpuMesh;
    use open_nfs_render::pipeline::{UiPipeline, VehiclePipeline};
    use open_nfs_render::surface::{SurfaceManager, DEPTH_FORMAT};
    use open_nfs_render::texture::{GpuTexture, TextureManager};

    log_android("=== Open-NFS Native Engine Started ===");

    let mut context: Option<GpuContext> = None;
    let mut surface_mgr: Option<SurfaceManager<'static>> = None;
    let mut vehicle_pipeline: Option<VehiclePipeline> = None;
    let mut ui_pipeline: Option<UiPipeline> = None;
    let mut mesh: Option<GpuMesh> = None;
    let mut tex_mgr: Option<TextureManager> = None;
    let mut camera = OrbitCamera::default();
    camera.yaw = 2.55;
    camera.pitch = 0.24;
    let mut touch_mgr = TouchManager::new();
    let mut diagnostics = RenderDiagnostics::new();
    let mut is_running = true;
    let mut frame_count: u64 = 0;

    while is_running {
        // When surface is ready, poll with 0ms timeout for full 60/120Hz presentation pacing.
        // When waiting for window/suspended, block with None to avoid 100% CPU lockup and ANR.
        let timeout = if surface_mgr.as_ref().map_or(false, |s| s.is_ready) {
            Some(std::time::Duration::from_millis(0))
        } else {
            None
        };

        app.poll_events(timeout, |event| match event {
            PollEvent::Main(MainEvent::InitWindow { .. }) => {
                log_android("--> PollEvent::InitWindow received");
                if let Some(native_window) = app.native_window() {
                    let width = native_window.width() as u32;
                    let height = native_window.height() as u32;
                    log_android(&format!("--> Window dimensions: {}x{}", width, height));

                    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
                        backends: wgpu::Backends::VULKAN,
                        ..Default::default()
                    });

                    let surface = match create_android_surface(&instance, &native_window) {
                        Ok(s) => {
                            log_android("--> Vulkan surface created successfully");
                            s
                        }
                        Err(e) => {
                            log_android(&format!(
                                "ERROR: Failed to create Android surface: {:?}",
                                e
                            ));
                            return;
                        }
                    };

                    let ctx = match pollster::block_on(GpuContext::new_with_surface(
                        instance, &surface,
                    )) {
                        Ok(c) => {
                            log_android(&format!(
                                "--> GPU Context initialized: {} ({})",
                                c.capabilities.device_name, c.capabilities.backend
                            ));
                            c
                        }
                        Err(e) => {
                            log_android(&format!(
                                "ERROR: Failed to initialize GpuContext: {:?}",
                                e
                            ));
                            return;
                        }
                    };

                    let sm = SurfaceManager::new(surface, &ctx.adapter, &ctx.device, width, height);
                    log_android(&format!(
                        "--> Swapchain configured: {}x{}, format={:?}",
                        sm.width, sm.height, sm.config.format
                    ));

                    let vp =
                        VehiclePipeline::new(&ctx.device, sm.config.format, Some(DEPTH_FORMAT));
                    let up = UiPipeline::new(&ctx.device, sm.config.format, Some(DEPTH_FORMAT));

                    let fallback = GpuTexture::create_fallback_white(
                        &ctx.device,
                        &ctx.queue,
                        &vp.texture_layout,
                    );
                    let mut tm = TextureManager::new(fallback);

                    if let Some(mut tex_asset) = app
                        .asset_manager()
                        .open(&std::ffi::CString::new("textures/350Z.bin").unwrap())
                    {
                        log_android("--> Reading textures/350Z.bin from APK assets...");
                        let mut tex_bytes = Vec::new();
                        if let Ok(buf) = tex_asset.buffer() {
                            if !buf.is_empty() {
                                tex_bytes.extend_from_slice(buf);
                            }
                        }
                        if tex_bytes.is_empty() {
                            use std::io::Read;
                            let _ = tex_asset.read_to_end(&mut tex_bytes);
                        }
                        log_android(&format!(
                            "--> Read {} bytes of textures/350Z.bin",
                            tex_bytes.len()
                        ));

                        if let Ok(pack) = open_nfs_asset::texture::parse_texture_pack(&tex_bytes) {
                            let mut loaded_count = 0;
                            for entry in pack.textures {
                                if let Ok(rgba) = entry.decode_to_rgba8() {
                                    let gpu_tex = GpuTexture::from_rgba8(
                                        &ctx.device,
                                        &ctx.queue,
                                        &vp.texture_layout,
                                        entry.width as u32,
                                        entry.height as u32,
                                        &rgba,
                                        Some(&format!("Tex 0x{:08X}", entry.hash)),
                                    );
                                    tm.insert(entry.hash, gpu_tex);
                                    loaded_count += 1;
                                }
                            }
                            log_android(&format!(
                                "--> Loaded {} textures successfully",
                                loaded_count
                            ));
                        }
                    }
                    tm.register_nfsu2_materials(&ctx.device, &ctx.queue, &vp.texture_layout);

                    let model = if let Some(mut model_asset) = app
                        .asset_manager()
                        .open(&std::ffi::CString::new("models/350Z.onfs-model").unwrap())
                    {
                        log_android("--> Reading models/350Z.onfs-model from APK assets...");
                        let mut model_bytes = Vec::new();
                        if let Ok(buf) = model_asset.buffer() {
                            if !buf.is_empty() {
                                model_bytes.extend_from_slice(buf);
                            }
                        }
                        if model_bytes.is_empty() {
                            use std::io::Read;
                            let _ = model_asset.read_to_end(&mut model_bytes);
                        }
                        log_android(&format!(
                            "--> Read {} bytes of 350Z.onfs-model",
                            model_bytes.len()
                        ));

                        match open_nfs_asset_runtime::format::deserialize_runtime_model(
                            &model_bytes,
                        ) {
                            Ok(m) => {
                                log_android(&format!(
                                    "--> Loaded 350Z model ({} parts)",
                                    m.parts.len()
                                ));
                                m
                            }
                            Err(e) => {
                                log_android(&format!(
                                    "WARN: Model parse error {:?}, using procedural fallback",
                                    e
                                ));
                                generate_procedural_car()
                            }
                        }
                    } else {
                        log_android(
                            "--> No bundled 350Z model found, generating procedural fallback",
                        );
                        generate_procedural_car()
                    };

                    let m = GpuMesh::from_runtime_model(&ctx.device, &model, "350Z Stock Car");
                    log_android(&format!(
                        "--> Assembled Stock Car Mesh: {} vertices, {} indices, {} submeshes",
                        m.num_vertices,
                        m.num_indices,
                        m.submeshes.len()
                    ));

                    touch_mgr.resize(width, height);
                    camera.set_aspect(width, height);

                    context = Some(ctx);
                    surface_mgr = Some(sm);
                    vehicle_pipeline = Some(vp);
                    ui_pipeline = Some(up);
                    mesh = Some(m);
                    tex_mgr = Some(tm);
                    log_android("--> Initialization complete! Entering render loop.");
                }
            }
            PollEvent::Main(
                MainEvent::WindowResized { .. }
                | MainEvent::ConfigChanged { .. }
                | MainEvent::RedrawNeeded { .. },
            ) => {
                if let (Some(ctx), Some(sm), Some(win)) =
                    (&context, &mut surface_mgr, app.native_window())
                {
                    let w = win.width() as u32;
                    let h = win.height() as u32;
                    if w > 0 && h > 0 && (w != sm.width || h != sm.height) {
                        sm.resize(&ctx.device, w, h);
                        camera.set_aspect(w, h);
                        touch_mgr.resize(w, h);
                        log_android(&format!("--> Window resized/reconfigured to {}x{}", w, h));
                    }
                }
            }

            PollEvent::Main(MainEvent::Pause) => {
                log_android("--> MainEvent::Pause received");
                touch_mgr.cancel_all_touches();
                if let (Some(ctx), Some(sm)) = (&context, &mut surface_mgr) {
                    sm.resize(&ctx.device, 0, 0);
                }
            }
            PollEvent::Main(MainEvent::Resume { .. }) => {
                log_android("--> MainEvent::Resume received");
                if let (Some(ctx), Some(sm), Some(win)) =
                    (&context, &mut surface_mgr, app.native_window())
                {
                    sm.resize(&ctx.device, win.width() as u32, win.height() as u32);
                }
            }
            PollEvent::Main(MainEvent::TerminateWindow { .. }) => {
                log_android("--> MainEvent::TerminateWindow received");
                surface_mgr = None;
            }
            PollEvent::Main(MainEvent::Destroy) => {
                log_android("--> MainEvent::Destroy received");
                is_running = false;
            }
            _ => {}
        });

        // Drain pending input events to acknowledge touch interactions and prevent ANR
        if let Ok(mut iter) = app.input_events_iter() {
            while iter.next(|event| match event {
                InputEvent::MotionEvent(motion) => {
                    let action = motion.action();
                    match action {
                        MotionAction::Down | MotionAction::PointerDown => {
                            let p_idx = motion.pointer_index();
                            if let Some(pointer) = motion.pointers().nth(p_idx) {
                                touch_mgr.on_pointer_down(
                                    pointer.pointer_id() as u64,
                                    pointer.x(),
                                    pointer.y(),
                                );
                            }
                        }
                        MotionAction::Move => {
                            for pointer in motion.pointers() {
                                touch_mgr.on_pointer_move(
                                    pointer.pointer_id() as u64,
                                    pointer.x(),
                                    pointer.y(),
                                );
                            }
                        }
                        MotionAction::Up | MotionAction::PointerUp => {
                            let p_idx = motion.pointer_index();
                            if let Some(pointer) = motion.pointers().nth(p_idx) {
                                touch_mgr.on_pointer_up(pointer.pointer_id() as u64);
                            }
                        }
                        MotionAction::Cancel => {
                            touch_mgr.cancel_all_touches();
                        }
                        _ => {}
                    }
                    InputStatus::Handled
                }
                _ => InputStatus::Unhandled,
            }) {}
        }

        // Apply camera touch drag delta if user dragged the background viewport
        if let Some((dx, dy)) = touch_mgr.take_camera_drag() {
            camera.rotate(dx * 0.005, dy * 0.005);
        }

        if let (Some(ctx), Some(sm), Some(vp), Some(up), Some(m), Some(tm)) = (
            &context,
            &mut surface_mgr,
            &vehicle_pipeline,
            &ui_pipeline,
            &mesh,
            &tex_mgr,
        ) {
            if let Some(win) = app.native_window() {
                let cur_w = win.width() as u32;
                let cur_h = win.height() as u32;
                if cur_w > 0 && cur_h > 0 && (cur_w != sm.width || cur_h != sm.height) {
                    sm.resize(&ctx.device, cur_w, cur_h);
                    camera.set_aspect(cur_w, cur_h);
                    touch_mgr.resize(cur_w, cur_h);
                    log_android(&format!(
                        "--> Adapted surface to window size: {}x{}",
                        cur_w, cur_h
                    ));
                }
            }

            if sm.is_ready {
                let dt = diagnostics.on_frame_start();
                touch_mgr.tick(dt);

                let output = match sm.acquire() {
                    Ok(out) => out,
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        sm.reconfigure(&ctx.device);
                        continue;
                    }
                    Err(wgpu::SurfaceError::Timeout) => {
                        continue;
                    }
                    Err(e) => {
                        log_android(&format!("WARN: Surface acquire error: {:?}", e));
                        continue;
                    }
                };

                let view = output
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let camera_uniforms = camera.build_uniforms();
                vp.camera_uniforms.update(&ctx.queue, &camera_uniforms);
                up.update_screen_size(&ctx.queue, sm.width as f32, sm.height as f32);

                let mut encoder =
                    ctx.device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Android Frame Encoder"),
                        });

                {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Android Render Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
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
                            view: &sm.depth_view,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                store: wgpu::StoreOp::Discard,
                            }),
                            stencil_ops: None,
                        }),
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

                    vp.render_mesh(&mut rpass, m, tm);
                    up.render_controls(&ctx.queue, &mut rpass, &touch_mgr);
                }

                ctx.queue.submit(Some(encoder.finish()));
                output.present();

                frame_count += 1;
                if frame_count <= 5 || frame_count % 300 == 0 {
                    log_android(&format!(
                        "--> Frame presented: #{}, FPS: {:.1}, FrameTime: {:.2}ms",
                        frame_count,
                        diagnostics.average_fps(),
                        diagnostics.avg_frame_time_ms()
                    ));
                }
            }
        }
    }
    log_android("=== Open-NFS Native Engine Exiting ===");
}
