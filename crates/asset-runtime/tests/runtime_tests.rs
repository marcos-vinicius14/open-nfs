use open_nfs_asset_runtime::error::RuntimeAssetError;
use open_nfs_asset_runtime::format::{
    deserialize_runtime_model, serialize_runtime_model, ONFM_MAGIC, ONFM_VERSION,
};
use open_nfs_asset_runtime::model::{
    RuntimeBounds, RuntimeModel, RuntimePart, RuntimeSubmesh, RuntimeVertex,
};

fn create_sample_runtime_model() -> RuntimeModel {
    RuntimeModel {
        source_sha256: [0xAB; 32],
        parts: vec![RuntimePart {
            name: "CUBE_PART".to_string(),
            lod_level: 0,
            part_hash: 0x1234_5678,
            bounds: RuntimeBounds {
                min: [-1.0, -1.0, -1.0],
                max: [1.0, 1.0, 1.0],
            },
            vertices: vec![
                RuntimeVertex {
                    position: [-1.0, -1.0, -1.0],
                    normal: [0.0, 1.0, 0.0],
                    color: 0xFFFF_FFFF,
                    uv: [0.0, 0.0],
                },
                RuntimeVertex {
                    position: [1.0, -1.0, -1.0],
                    normal: [0.0, 1.0, 0.0],
                    color: 0xFFFF_FFFF,
                    uv: [1.0, 0.0],
                },
                RuntimeVertex {
                    position: [1.0, 1.0, -1.0],
                    normal: [0.0, 1.0, 0.0],
                    color: 0xFFFF_FFFF,
                    uv: [1.0, 1.0],
                },
            ],
            indices: vec![0, 1, 2],
            submeshes: vec![RuntimeSubmesh {
                material_id: 42,
                index_offset: 0,
                index_count: 3,
                shader_flags: 1,
                bounds: RuntimeBounds {
                    min: [-1.0, -1.0, -1.0],
                    max: [1.0, 1.0, -1.0],
                },
            }],
        }],
    }
}

#[test]
fn test_roundtrip_serialization() {
    let original = create_sample_runtime_model();
    let bytes = serialize_runtime_model(&original);

    assert_eq!(&bytes[0..4], &ONFM_MAGIC);
    assert_eq!(
        u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
        ONFM_VERSION
    );

    let parsed = deserialize_runtime_model(&bytes).expect("Failed to deserialize valid ONFM");
    assert_eq!(parsed, original);
}

#[test]
fn test_deserialize_from_bytes_and_file() {
    let original = create_sample_runtime_model();
    let bytes = serialize_runtime_model(&original);

    // 1. In-memory slice
    let in_memory = deserialize_runtime_model(&bytes).expect("Failed to deserialize from slice");
    assert_eq!(in_memory, original);

    // 2. From file on disk
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join("test_model.onfs-model");
    std::fs::write(&file_path, &bytes).expect("Failed to write temporary test file");

    let file_bytes = std::fs::read(&file_path).expect("Failed to read test file");
    let file_model =
        deserialize_runtime_model(&file_bytes).expect("Failed to deserialize file bytes");
    assert_eq!(file_model, original);

    let _ = std::fs::remove_file(file_path);
}

#[test]
fn test_corrupt_magic_fails() {
    let mut bytes = serialize_runtime_model(&create_sample_runtime_model());
    bytes[0] = b'X';
    let err = deserialize_runtime_model(&bytes).expect_err("Should fail with bad magic");
    assert!(matches!(err, RuntimeAssetError::InvalidMagic(_)));
}

#[test]
fn test_corrupt_version_fails() {
    let mut bytes = serialize_runtime_model(&create_sample_runtime_model());
    bytes[4] = 99; // Version 99
    let err = deserialize_runtime_model(&bytes).expect_err("Should fail with bad version");
    assert!(matches!(err, RuntimeAssetError::UnsupportedVersion { .. }));
}

#[test]
fn test_truncated_bytes_fails() {
    let bytes = serialize_runtime_model(&create_sample_runtime_model());
    for cut in 0..bytes.len() - 1 {
        let err = deserialize_runtime_model(&bytes[..cut]);
        assert!(err.is_err());
    }
}

#[test]
fn test_out_of_bounds_index_fails() {
    let mut model = create_sample_runtime_model();
    // Vertex count is 3 (indices 0, 1, 2). Put an index 5
    model.parts[0].indices.push(5);
    let bytes = serialize_runtime_model(&model);
    let err = deserialize_runtime_model(&bytes).expect_err("Should fail with out of bounds index");
    assert!(matches!(
        err,
        RuntimeAssetError::IndexOutOfBounds {
            index: 5,
            vertex_count: 3
        }
    ));
}

#[test]
fn test_load_converted_350z_if_present() {
    let path = std::path::Path::new("/tmp/350Z.onfs-model");
    if !path.exists() {
        return;
    }
    let bytes = std::fs::read(path).expect("Failed to read 350Z model file");
    let model = deserialize_runtime_model(&bytes).expect("Failed to load converted 350Z model");
    assert_eq!(model.parts.len(), 607);
    assert_eq!(model.total_vertices(), 140132);
    assert_eq!(model.total_indices(), 329448);
    let base = model
        .find_part("350Z_BASE_A")
        .expect("350Z_BASE_A missing in loaded model");
    assert_eq!(base.vertices.len(), 657);
    assert_eq!(base.indices.len(), 1500);
}

#[test]
fn test_unaligned_slice_safety() {
    let original = create_sample_runtime_model();
    let bytes = serialize_runtime_model(&original);

    for pad in 1..=3 {
        let mut unaligned = vec![0xAA; pad];
        unaligned.extend_from_slice(&bytes);

        let unaligned_slice = &unaligned[pad..];
        let parsed = deserialize_runtime_model(unaligned_slice)
            .expect("Should deserialize unaligned slice without panic");
        assert_eq!(parsed, original);

        let err = deserialize_runtime_model(&unaligned[..pad + 20]);
        assert!(err.is_err());
    }
}
