#![deny(unsafe_code)]

#[cfg(not(target_os = "android"))]
mod app {
    use open_nfs_asset::geometry::{parse_geometry, ParsedGeometryArchive};
    use open_nfs_asset::texture::parse_texture_pack;
    use open_nfs_asset_runtime::model::{RuntimeBounds, RuntimeModel, RuntimePart, RuntimeSubmesh};
    use open_nfs_render::camera::OrbitCamera;
    use open_nfs_render::capabilities::GpuCapabilitiesReport;
    use open_nfs_render::context::GpuContext;
    use open_nfs_render::controls::TouchManager;
    use open_nfs_render::diagnostics::RenderDiagnostics;
    use open_nfs_render::mesh::GpuMesh;
    use open_nfs_render::offscreen::OffscreenRenderer;
    use open_nfs_render::pipeline::{UiPipeline, VehiclePipeline};
    use open_nfs_render::surface::{SurfaceManager, DEPTH_FORMAT};
    use open_nfs_render::texture::{GpuTexture, TextureManager};
    use std::fs;
    use std::path::Path;
    use std::sync::Arc;
    use std::time::Instant;
    use winit::application::ApplicationHandler;
    use winit::dpi::LogicalSize;
    use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
    use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
    use winit::keyboard::{KeyCode, PhysicalKey};
    use winit::window::{Window, WindowId};

    struct ViewerApp {
        window: Option<Arc<Window>>,
        context: Option<GpuContext>,
        surface_mgr: Option<SurfaceManager<'static>>,
        vehicle_pipeline: Option<VehiclePipeline>,
        ui_pipeline: Option<UiPipeline>,
        mesh: Option<GpuMesh>,
        texture_mgr: Option<TextureManager>,
        camera: OrbitCamera,
        touch_mgr: TouchManager,
        diagnostics: RenderDiagnostics,
        is_left_mouse_down: bool,
        last_cursor_pos: Option<(f64, f64)>,
        caps_report: Option<GpuCapabilitiesReport>,
    }

    impl Default for ViewerApp {
        fn default() -> Self {
            Self {
                window: None,
                context: None,
                surface_mgr: None,
                vehicle_pipeline: None,
                ui_pipeline: None,
                mesh: None,
                texture_mgr: None,
                camera: OrbitCamera::default(),
                touch_mgr: TouchManager::new(),
                diagnostics: RenderDiagnostics::new(),
                is_left_mouse_down: false,
                last_cursor_pos: None,
                caps_report: None,
            }
        }
    }

    impl ViewerApp {
        fn init_gpu(&mut self, window: Arc<Window>) {
            let size = window.inner_size();
            let width = size.width.max(1);
            let height = size.height.max(1);

            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
                backends: wgpu::Backends::VULKAN | wgpu::Backends::GL,
                ..Default::default()
            });

            // Safe surface creation from Arc<Window>
            let surface = instance.create_surface(window.clone()).unwrap();

            let context =
                pollster::block_on(GpuContext::new_with_surface(instance, &surface)).unwrap();
            let surface_mgr =
                SurfaceManager::new(surface, &context.adapter, &context.device, width, height);

            let vehicle_pipeline = VehiclePipeline::new(
                &context.device,
                surface_mgr.config.format,
                Some(DEPTH_FORMAT),
            );
            let ui_pipeline = UiPipeline::new(
                &context.device,
                surface_mgr.config.format,
                Some(DEPTH_FORMAT),
            );

            // Probe capabilities
            let caps = GpuCapabilitiesReport::probe(&context.adapter);
            println!("\n=== GPU Adapter Initialized ===");
            println!("{}", caps.to_markdown());
            self.caps_report = Some(caps);

            // Load 350Z asset or procedural fallback
            let load_start = Instant::now();
            let (mesh, tex_mgr) = load_vehicle_assets(&context, &vehicle_pipeline.texture_layout);
            let upload_duration = load_start.elapsed().as_secs_f32() * 1000.0;
            self.diagnostics.asset_upload_time_ms = upload_duration;
            self.diagnostics.textures_loaded = tex_mgr.len() as u32;
            self.diagnostics
                .set_frame_counters(mesh.submeshes.len().max(1) as u32, mesh.num_indices / 3);

            println!(
            "Loaded model: {} vertices, {} indices, {} submeshes, {} textures (uploaded in {:.2} ms)",
            mesh.num_vertices,
            mesh.num_indices,
            mesh.submeshes.len(),
            tex_mgr.len(),
            upload_duration
        );

            self.touch_mgr.resize(width, height);
            self.camera.set_aspect(width, height);

            self.window = Some(window);
            self.context = Some(context);
            self.surface_mgr = Some(surface_mgr);
            self.vehicle_pipeline = Some(vehicle_pipeline);
            self.ui_pipeline = Some(ui_pipeline);
            self.mesh = Some(mesh);
            self.texture_mgr = Some(tex_mgr);
        }
    }

    impl ApplicationHandler for ViewerApp {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            if self.window.is_none() {
                let win_attrs = Window::default_attributes()
                    .with_title("Open-NFS M3 — Nissan 350Z Desktop Viewer")
                    .with_inner_size(LogicalSize::new(1280, 720));

                let window = Arc::new(event_loop.create_window(win_attrs).unwrap());
                self.init_gpu(window);
            }
        }

        fn window_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            _window_id: WindowId,
            event: WindowEvent,
        ) {
            match event {
                WindowEvent::CloseRequested => {
                    event_loop.exit();
                }
                WindowEvent::Resized(new_size) => {
                    if let (Some(context), Some(surface_mgr)) =
                        (&self.context, &mut self.surface_mgr)
                    {
                        surface_mgr.resize(&context.device, new_size.width, new_size.height);
                        self.camera.set_aspect(new_size.width, new_size.height);
                        self.touch_mgr.resize(new_size.width, new_size.height);
                    }
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    let zoom_factor = match delta {
                        MouseScrollDelta::LineDelta(_x, y) => {
                            if y > 0.0 {
                                0.9
                            } else {
                                1.1
                            }
                        }
                        MouseScrollDelta::PixelDelta(pos) => {
                            if pos.y > 0.0 {
                                0.95
                            } else {
                                1.05
                            }
                        }
                    };
                    self.camera.zoom(zoom_factor);
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    if button == MouseButton::Left {
                        match state {
                            ElementState::Pressed => {
                                self.is_left_mouse_down = true;
                                if let Some((x, y)) = self.last_cursor_pos {
                                    self.touch_mgr.on_pointer_down(1, x as f32, y as f32);
                                }
                            }
                            ElementState::Released => {
                                self.is_left_mouse_down = false;
                                self.touch_mgr.on_pointer_up(1);
                            }
                        }
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let prev_pos = self.last_cursor_pos;
                    self.last_cursor_pos = Some((position.x, position.y));

                    if self.is_left_mouse_down {
                        self.touch_mgr
                            .on_pointer_move(1, position.x as f32, position.y as f32);

                        // If not interacting with virtual controls, orbit camera
                        if let Some((dx, dy)) = self.touch_mgr.take_camera_drag() {
                            self.camera.rotate(dx * 0.005, dy * 0.005);
                        } else if let Some((px, py)) = prev_pos {
                            if !self.touch_mgr.steering.is_active
                                && self.touch_mgr.pedals.throttle_pointer_id.is_none()
                                && self.touch_mgr.pedals.brake_pointer_id.is_none()
                            {
                                let dx = (position.x - px) as f32;
                                let dy = (position.y - py) as f32;
                                self.camera.rotate(dx * 0.005, dy * 0.005);
                            }
                        }
                    }
                }
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            physical_key: PhysicalKey::Code(code),
                            state,
                            ..
                        },
                    ..
                } => {
                    let pressed = state == ElementState::Pressed;
                    match code {
                        KeyCode::ArrowLeft | KeyCode::KeyA => {
                            if pressed {
                                self.touch_mgr.steering.current_angle_rad =
                                    -self.touch_mgr.steering.max_angle_rad * 0.8;
                                self.touch_mgr.steering.is_active = true;
                            } else {
                                self.touch_mgr.steering.is_active = false;
                            }
                        }
                        KeyCode::ArrowRight | KeyCode::KeyD => {
                            if pressed {
                                self.touch_mgr.steering.current_angle_rad =
                                    self.touch_mgr.steering.max_angle_rad * 0.8;
                                self.touch_mgr.steering.is_active = true;
                            } else {
                                self.touch_mgr.steering.is_active = false;
                            }
                        }
                        KeyCode::ArrowUp | KeyCode::KeyW => {
                            self.touch_mgr.pedals.throttle_value = if pressed { 1.0 } else { 0.0 };
                        }
                        KeyCode::ArrowDown | KeyCode::KeyS | KeyCode::Space => {
                            self.touch_mgr.pedals.brake_value = if pressed { 1.0 } else { 0.0 };
                        }
                        KeyCode::KeyR => {
                            if pressed {
                                self.camera.reset();
                                println!("Camera reset to default view.");
                            }
                        }
                        KeyCode::KeyP => {
                            if pressed {
                                if let Some(caps) = &self.caps_report {
                                    let md =
                                        self.diagnostics.generate_baseline_markdown(caps, 110.5);
                                    println!("\n{md}");
                                    let _ = fs::write("target/desktop_baseline.md", md);
                                    println!("Saved report to target/desktop_baseline.md");
                                }
                            }
                        }
                        KeyCode::F12 | KeyCode::KeyC => {
                            if pressed {
                                self.capture_screenshot();
                            }
                        }
                        KeyCode::Escape => {
                            event_loop.exit();
                        }
                        _ => {}
                    }
                }
                WindowEvent::RedrawRequested => {
                    self.render_frame();
                }
                _ => {}
            }
        }

        fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }

    impl ViewerApp {
        fn render_frame(&mut self) {
            let dt = self.diagnostics.on_frame_start();
            self.touch_mgr.tick(dt);

            let (context, surface_mgr, vehicle_pipeline, ui_pipeline, mesh, texture_mgr) = match (
                &self.context,
                &mut self.surface_mgr,
                &self.vehicle_pipeline,
                &self.ui_pipeline,
                &self.mesh,
                &self.texture_mgr,
            ) {
                (Some(c), Some(s), Some(vp), Some(up), Some(m), Some(t)) => (c, s, vp, up, m, t),
                _ => return,
            };

            if !surface_mgr.is_ready {
                return;
            }

            let output = match surface_mgr.acquire() {
                Ok(output) => output,
                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                    surface_mgr.reconfigure(&context.device);
                    return;
                }
                Err(wgpu::SurfaceError::OutOfMemory) => {
                    eprintln!("GPU Out of memory!");
                    return;
                }
                Err(e) => {
                    eprintln!("Surface acquire error: {:?}", e);
                    return;
                }
            };

            let view = output
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            // Update uniforms
            let camera_uniforms = self.camera.build_uniforms();
            vehicle_pipeline
                .camera_uniforms
                .update(&context.queue, &camera_uniforms);

            ui_pipeline.update_screen_size(
                &context.queue,
                surface_mgr.width as f32,
                surface_mgr.height as f32,
            );

            let mut encoder =
                context
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Frame Command Encoder"),
                    });

            {
                let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Main Vehicle Render Pass"),
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
                        view: &surface_mgr.depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Discard,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                // 1. Draw 3D Vehicle
                vehicle_pipeline.render_mesh(&mut rpass, mesh, texture_mgr);

                // 2. Draw 2D Virtual Touch Controls Overlay
                ui_pipeline.render_controls(&context.queue, &mut rpass, &self.touch_mgr);
            }

            context.queue.submit(Some(encoder.finish()));
            output.present();
        }

        fn capture_screenshot(&mut self) {
            if let (Some(context), Some(mesh), Some(tex_mgr)) =
                (&self.context, &self.mesh, &self.texture_mgr)
            {
                let mut offscreen = OffscreenRenderer::new(context, 1280, 720);
                let pixels = pollster::block_on(offscreen.render_to_rgba8(
                    context,
                    mesh,
                    tex_mgr,
                    &self.camera,
                    Some(&self.touch_mgr),
                ));
                let out_path = Path::new("target/m3_screenshot.ppm");
                if let Err(e) = OffscreenRenderer::save_ppm(out_path, 1280, 720, &pixels) {
                    eprintln!("Failed to save screenshot: {e}");
                } else {
                    println!("Screenshot successfully saved to target/m3_screenshot.ppm");
                }
            }
        }
    }

    /// Convert parsed geometry archive into a strongly typed RuntimeModel.
    fn geometry_to_runtime_model(
        archive: ParsedGeometryArchive,
        source_sha256: [u8; 32],
    ) -> RuntimeModel {
        let mut runtime_parts = Vec::with_capacity(archive.parts.len());
        for part in archive.parts {
            let submeshes: Vec<RuntimeSubmesh> = part
                .submeshes
                .into_iter()
                .map(|sm| RuntimeSubmesh {
                    material_id: sm.material_id,
                    index_offset: sm.index_offset,
                    index_count: sm.index_count,
                    shader_flags: sm.shader_flags,
                    bounds: RuntimeBounds {
                        min: sm.bbox_min,
                        max: sm.bbox_max,
                    },
                })
                .collect();

            runtime_parts.push(RuntimePart {
                name: part.header.name,
                lod_level: part.header.lod_level,
                part_hash: part.header.part_hash,
                bounds: RuntimeBounds {
                    min: part.header.bbox_min,
                    max: part.header.bbox_max,
                },
                vertices: part.vertices,
                indices: part.indices,
                submeshes,
            });
        }

        RuntimeModel {
            source_sha256,
            parts: runtime_parts,
        }
    }

    /// Load car model from disk or generate a clean synthetic sports car if game assets are absent.
    fn load_vehicle_assets(
        context: &GpuContext,
        texture_layout: &wgpu::BindGroupLayout,
    ) -> (GpuMesh, TextureManager) {
        let fallback =
            GpuTexture::create_fallback_white(&context.device, &context.queue, texture_layout);
        let mut tex_mgr = TextureManager::new(fallback);

        let geom_path = Path::new("game-assets/CARS/350Z/GEOMETRY.BIN");
        let tex_path = Path::new("game-assets/CARS/350Z/TEXTURES.BIN");

        if geom_path.exists() {
            println!("Loading 350Z car assets from {:?}", geom_path);
            let geom_bytes = fs::read(geom_path).expect("Failed to read GEOMETRY.BIN");
            let archive = parse_geometry(&geom_bytes).expect("Failed to parse GEOMETRY.BIN");
            let model = geometry_to_runtime_model(archive, [0u8; 32]);

            if tex_path.exists() {
                println!("Loading 350Z textures from {:?}", tex_path);
                let tex_bytes = fs::read(tex_path).expect("Failed to read TEXTURES.BIN");
                match parse_texture_pack(&tex_bytes) {
                    Ok(pack) => {
                        for entry in pack.textures {
                            match entry.decode_to_rgba8() {
                                Ok(rgba) => {
                                    let gpu_tex = GpuTexture::from_rgba8(
                                        &context.device,
                                        &context.queue,
                                        texture_layout,
                                        entry.width as u32,
                                        entry.height as u32,
                                        &rgba,
                                        Some(&format!("Tex 0x{:08X}", entry.hash)),
                                    );
                                    tex_mgr.insert(entry.hash, gpu_tex);
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        hash = entry.hash,
                                        "Failed decoding texture: {e}"
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed parsing texture pack: {e}");
                    }
                }
            }
            tex_mgr.register_nfsu2_materials(&context.device, &context.queue, texture_layout);

            let gpu_mesh = GpuMesh::from_runtime_model(&context.device, &model, "350Z Stock Car");
            (gpu_mesh, tex_mgr)
        } else {
            println!("No local 350Z assets found, generating procedural sports car model.");
            let model = generate_procedural_car();
            let gpu_mesh = GpuMesh::from_runtime_model(&context.device, &model, "Procedural Car");
            (gpu_mesh, tex_mgr)
        }
    }

    use open_nfs_android_host::generate_procedural_car;

    pub fn run() {
        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);

        let mut app = ViewerApp::default();
        event_loop.run_app(&mut app).unwrap();
    }
} // mod app

#[cfg(not(target_os = "android"))]
fn main() {
    app::run();
}

#[cfg(target_os = "android")]
fn main() {}
