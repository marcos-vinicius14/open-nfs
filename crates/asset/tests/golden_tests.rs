use open_nfs_asset::geometry::parse_geometry;
use open_nfs_asset::synthetic::build_synthetic_cube_geometry;
use std::path::Path;

#[test]
fn test_synthetic_cube_golden_parse() {
    let raw_bytes = build_synthetic_cube_geometry();
    let archive = parse_geometry(&raw_bytes).expect("Failed to parse synthetic cube geometry");

    assert_eq!(archive.parts.len(), 1);
    let part = &archive.parts[0];
    assert_eq!(part.header.name, "SYNTHETIC_CUBE_A");
    assert_eq!(part.vertices.len(), 8);
    assert_eq!(part.indices.len(), 36);
    assert_eq!(part.submeshes.len(), 1);

    // Verify submesh slice bounds
    let submesh = &part.submeshes[0];
    assert_eq!(submesh.index_count, 36);
    assert_eq!(submesh.index_offset, 0);

    // Verify vertex attributes
    for v in &part.vertices {
        let normal_len = (v.normal[0].powi(2) + v.normal[1].powi(2) + v.normal[2].powi(2)).sqrt();
        assert!((normal_len - 1.0).abs() < 1e-4);
    }
}

#[test]
fn test_local_350z_asset_if_present() {
    let candidates = [
        Path::new("game-assets/CARS/350Z/GEOMETRY.BIN"),
        Path::new("../../game-assets/CARS/350Z/GEOMETRY.BIN"),
    ];
    let path = match candidates.iter().find(|p| p.exists()) {
        Some(p) => *p,
        None => {
            eprintln!("Local game-assets/CARS/350Z/GEOMETRY.BIN not present; skipping local test.");
            return;
        }
    };

    let raw_bytes = std::fs::read(path).expect("Failed to read local 350Z asset");
    let archive = parse_geometry(&raw_bytes).expect("Failed to parse local 350Z GEOMETRY.BIN");

    assert!(!archive.parts.is_empty());

    // Search for base body
    let base = archive
        .find_part("350Z_BASE_A")
        .expect("Missing 350Z_BASE_A part in local asset");

    assert_eq!(base.vertices.len(), 657);
    assert_eq!(base.indices.len(), 1500); // 500 triangles * 3 indices
    assert_eq!(base.submeshes.len(), 11);

    // Verify normals are unit length
    for (i, v) in base.vertices.iter().enumerate() {
        let normal_len = (v.normal[0].powi(2) + v.normal[1].powi(2) + v.normal[2].powi(2)).sqrt();
        assert!(
            (normal_len - 1.0).abs() < 0.05,
            "Vertex {} has non-unit normal: length = {}",
            i,
            normal_len
        );
    }

    // Verify all indices are valid
    for &idx in &base.indices {
        assert!((idx as usize) < base.vertices.len());
    }
}

#[test]
fn test_local_350z_textures_if_present() {
    let candidates = [
        Path::new("game-assets/CARS/350Z/TEXTURES.BIN"),
        Path::new("../../game-assets/CARS/350Z/TEXTURES.BIN"),
    ];
    let path = match candidates.iter().find(|p| p.exists()) {
        Some(p) => *p,
        None => {
            eprintln!("Local game-assets/CARS/350Z/TEXTURES.BIN not present; skipping local test.");
            return;
        }
    };

    let raw_bytes = std::fs::read(path).expect("Failed to read local 350Z TEXTURES.BIN");
    let pack = open_nfs_asset::texture::parse_texture_pack(&raw_bytes)
        .expect("Failed to parse 350Z TEXTURES.BIN");

    println!("Loaded texture package: {:?}", pack.package_name);
    println!("Total textures extracted: {}", pack.textures.len());
    assert!(!pack.textures.is_empty());

    // Find the 128x64 DXT3 texture we analyzed
    let tex = pack
        .textures
        .iter()
        .find(|t| t.width == 128 && t.height == 64)
        .expect("Failed to find 128x64 texture");

    assert_eq!(tex.format, open_nfs_asset::texture::TextureFormat::Dxt3);
    assert!(!tex.data.is_empty());

    // Decode to RGBA8
    let rgba = tex
        .decode_to_rgba8()
        .expect("Failed to decode DXT3 to RGBA8");
    assert_eq!(rgba.len(), 128 * 64 * 4);
}
