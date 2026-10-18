use open_nfs_asset_runtime::model::{RuntimeBounds, RuntimeSubmesh, RuntimeVertex};
use open_nfs_render::camera::OrbitCamera;
use open_nfs_render::capabilities::GpuCapabilitiesReport;
use open_nfs_render::context::GpuContext;
use open_nfs_render::controls::{TouchManager, TouchTarget};
use open_nfs_render::diagnostics::RenderDiagnostics;
use open_nfs_render::mesh::GpuMesh;
use open_nfs_render::offscreen::OffscreenRenderer;
use open_nfs_render::texture::{GpuTexture, TextureManager};

#[test]
fn test_camera_orbit_math() {
    let mut camera = OrbitCamera::new(16.0 / 9.0);
    assert_eq!(camera.aspect, 16.0 / 9.0);

    let eye_before = camera.eye_position();
    assert!(eye_before[2] > 0.0);

    // Rotate 90 degrees yaw
    camera.rotate(std::f32::consts::FRAC_PI_2, 0.0);
    let eye_after = camera.eye_position();
    assert!((eye_after[0] - eye_before[2]).abs() < 0.2);

    // Uniforms should produce finite matrices
    let uniforms = camera.build_uniforms();
    for row in uniforms.view_proj {
        for val in row {
            assert!(val.is_finite());
        }
    }
}

#[test]
fn test_multitouch_control_manager() {
    let mut touch = TouchManager::new();
    touch.resize(1920, 1080);

    // Test steering wheel hit
    let t1 = touch.on_pointer_down(10, touch.steering.center_x, touch.steering.center_y);
    assert_eq!(t1, TouchTarget::SteeringWheel);
    assert!(touch.steering.is_active);

    // Test throttle pedal hit
    let t2 = touch.on_pointer_down(
        20,
        touch.pedals.throttle_x + 5.0,
        touch.pedals.throttle_y + 5.0,
    );
    assert_eq!(t2, TouchTarget::ThrottlePedal);
    assert!(touch.pedals.throttle_pointer_id.is_some());

    // State reflects input
    let state = touch.vehicle_control_state();
    assert!(state.throttle > 0.0);

    // Release pointer 20
    touch.on_pointer_up(20);
    let state2 = touch.vehicle_control_state();
    assert_eq!(state2.throttle, 0.0);

    // App pause cancels all touches
    touch.cancel_all_touches();
    assert!(!touch.steering.is_active);
    assert_eq!(touch.steering.active_pointer_id, None);
}

#[test]
fn test_render_diagnostics_telemetry() {
    let mut diag = RenderDiagnostics::new();
    diag.set_frame_counters(14, 28_450);
    assert_eq!(diag.frame_draw_calls, 14);
    assert_eq!(diag.frame_triangles, 28_450);

    let caps = GpuCapabilitiesReport {
        device_name: "Mock Adreno 750".to_string(),
        vendor_id: 0x5143,
        device_id: 0x0750,
        device_type: "IntegratedGpu".to_string(),
        driver_info: "Qualcomm Vulkan 1.3".to_string(),
        backend: "Vulkan".to_string(),
        max_texture_dimension_2d: 16384,
        max_buffer_size: 1024 * 1024 * 1024,
        max_bind_groups: 8,
        max_vertex_buffers: 16,
        supports_bc_compression: true,
        supports_etc2_compression: true,
        supports_astc_compression: true,
        supports_timestamp_queries: true,
    };

    let md = diag.generate_baseline_markdown(&caps, 85.4);
    assert!(md.contains("Mock Adreno 750"));
    assert!(md.contains("Draw Calls per Frame:"));
    assert!(md.contains("`14`"));
}

#[test]
fn test_headless_offscreen_render_triangle() {
    pollster::block_on(async {
        let context = match GpuContext::new_headless().await {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Skipping headless render test (no GPU adapter available): {e}");
                return;
            }
        };

        // Create a simple 3D triangle mesh
        let vertices = [
            RuntimeVertex {
                position: [0.0, 0.5, 0.0],
                normal: [0.0, 0.0, 1.0],
                color: 0xFFFFFFFF,
                uv: [0.5, 0.0],
            },
            RuntimeVertex {
                position: [-0.5, -0.5, 0.0],
                normal: [0.0, 0.0, 1.0],
                color: 0xFFFFFFFF,
                uv: [0.0, 1.0],
            },
            RuntimeVertex {
                position: [0.5, -0.5, 0.0],
                normal: [0.0, 0.0, 1.0],
                color: 0xFFFFFFFF,
                uv: [1.0, 1.0],
            },
        ];
        let indices = [0u32, 1, 2];
        let submesh = RuntimeSubmesh {
            material_id: 0,
            index_offset: 0,
            index_count: 3,
            shader_flags: 0,
            bounds: RuntimeBounds::default(),
        };

        let mesh = GpuMesh::new(
            &context.device,
            "Test Triangle",
            &vertices,
            &indices,
            vec![submesh],
            RuntimeBounds::default(),
        );

        let tex_layout = GpuTexture::create_bind_group_layout(&context.device);
        let fallback =
            GpuTexture::create_fallback_white(&context.device, &context.queue, &tex_layout);
        let tex_mgr = TextureManager::new(fallback);

        let mut offscreen = OffscreenRenderer::new(&context, 128, 128);
        let camera = OrbitCamera::default();

        let pixels = offscreen
            .render_to_rgba8(&context, &mesh, &tex_mgr, &camera, None)
            .await;

        assert_eq!(pixels.len(), 128 * 128 * 4);

        // Verify that some pixels are rendered (not all clear color)
        let clear_r = (0.08 * 255.0) as u8;
        let clear_g = (0.08 * 255.0) as u8;
        let clear_b = (0.12 * 255.0) as u8;

        let mut non_clear_pixels = 0;
        for px in pixels.chunks_exact(4) {
            let dr = (px[0] as i32 - clear_r as i32).abs();
            let dg = (px[1] as i32 - clear_g as i32).abs();
            let db = (px[2] as i32 - clear_b as i32).abs();
            if dr > 10 || dg > 10 || db > 10 {
                non_clear_pixels += 1;
            }
        }

        assert!(
            non_clear_pixels > 0,
            "Expected triangle to be rendered, but found 0 non-background pixels"
        );
    });
}

#[test]
fn test_350z_full_render_and_export() {
    use open_nfs_asset::geometry::parse_geometry;
    use open_nfs_asset::texture::parse_texture_pack;
    use open_nfs_asset_runtime::model::{RuntimeBounds, RuntimeModel, RuntimePart, RuntimeSubmesh};
    use std::fs;
    use std::path::Path;

    let geom_path = Path::new("../../game-assets/CARS/350Z/GEOMETRY.BIN");
    let fallback_geom = Path::new("game-assets/CARS/350Z/GEOMETRY.BIN");
    let actual_geom = if geom_path.exists() {
        geom_path
    } else if fallback_geom.exists() {
        fallback_geom
    } else {
        eprintln!("Skipping 350Z test: assets not found in local workspace");
        return;
    };

    pollster::block_on(async {
        let context = match GpuContext::new_headless().await {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Skipping headless 350Z render test (no GPU adapter): {e}");
                return;
            }
        };

        let geom_bytes = fs::read(actual_geom).expect("read GEOMETRY.BIN");
        let archive = parse_geometry(&geom_bytes).expect("parse GEOMETRY.BIN");

        let mut parts = Vec::new();
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

            parts.push(RuntimePart {
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

        let model = RuntimeModel {
            source_sha256: [0u8; 32],
            parts,
        };

        let mesh = GpuMesh::from_runtime_model(&context.device, &model, "350Z Test Mesh");
        assert!(mesh.num_vertices > 0);
        assert!(mesh.num_indices > 0);

        let tex_layout = GpuTexture::create_bind_group_layout(&context.device);
        let fallback =
            GpuTexture::create_fallback_white(&context.device, &context.queue, &tex_layout);
        let mut tex_mgr = TextureManager::new(fallback);

        let tex_path = actual_geom.parent().unwrap().join("TEXTURES.BIN");
        if tex_path.exists() {
            let tex_bytes = fs::read(&tex_path).expect("read TEXTURES.BIN");
            if let Ok(pack) = parse_texture_pack(&tex_bytes) {
                for entry in pack.textures {
                    if let Ok(rgba) = entry.decode_to_rgba8() {
                        let gpu_tex = GpuTexture::from_rgba8(
                            &context.device,
                            &context.queue,
                            &tex_layout,
                            entry.width as u32,
                            entry.height as u32,
                            &rgba,
                            Some(&format!("Tex 0x{:08X}", entry.hash)),
                        );
                        tex_mgr.insert(entry.hash, gpu_tex);
                    }
                }
            }
        }
        tex_mgr.register_nfsu2_materials(&context.device, &context.queue, &tex_layout);

        let mut offscreen = OffscreenRenderer::new(&context, 1280, 720);
        let mut camera = OrbitCamera::new(1280.0 / 720.0);
        camera.yaw = 2.55;
        camera.pitch = 0.24;
        camera.distance = 4.8;

        let mut touch_mgr = TouchManager::new();
        touch_mgr.resize(1280, 720);
        // Press throttle slightly
        touch_mgr.pedals.throttle_value = 0.75;
        // Turn wheel slightly right
        touch_mgr.steering.current_angle_rad = 0.35;

        let pixels = offscreen
            .render_to_rgba8(&context, &mesh, &tex_mgr, &camera, Some(&touch_mgr))
            .await;

        assert_eq!(pixels.len(), 1280 * 720 * 4);

        // Check non-clear pixel count
        let clear_r = (0.08 * 255.0) as u8;
        let clear_g = (0.08 * 255.0) as u8;
        let clear_b = (0.12 * 255.0) as u8;

        let mut non_clear = 0;
        for px in pixels.chunks_exact(4) {
            let dr = (px[0] as i32 - clear_r as i32).abs();
            let dg = (px[1] as i32 - clear_g as i32).abs();
            let db = (px[2] as i32 - clear_b as i32).abs();
            if dr > 10 || dg > 10 || db > 10 {
                non_clear += 1;
            }
        }

        println!(
            "350Z Offscreen Render: {} non-background pixels detected out of {} total",
            non_clear,
            1280 * 720
        );
        assert!(
            non_clear > 10_000,
            "Vehicle and controls must render visible pixels"
        );

        // Save output artifact
        let _ = std::fs::create_dir_all("target");
        let ppm_path = Path::new("target/350z_rendered.ppm");
        if OffscreenRenderer::save_ppm(ppm_path, 1280, 720, &pixels).is_ok() {
            assert!(ppm_path.exists());
        } else {
            let tmp_ppm = std::env::temp_dir().join("350z_rendered.ppm");
            OffscreenRenderer::save_ppm(&tmp_ppm, 1280, 720, &pixels).unwrap();
            assert!(tmp_ppm.exists());
        }
    });
}
