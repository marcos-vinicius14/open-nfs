use open_nfs_asset::error::{ChunkError, GeometryError, TextureError};
use open_nfs_asset::geometry::parse_geometry;
use open_nfs_asset::synthetic::build_synthetic_cube_geometry;
use open_nfs_asset::texture::{decompress_jdlz, parse_texture_pack};

#[test]
fn test_geometry_empty_or_truncated() {
    assert!(matches!(
        parse_geometry(&[]),
        Err(GeometryError::Chunk(ChunkError::UnexpectedEof { .. }))
    ));

    let cube_bytes = build_synthetic_cube_geometry();
    // Test truncation at various byte offsets
    for cut in 1..cube_bytes.len().min(256) {
        let res = parse_geometry(&cube_bytes[..cut]);
        assert!(res.is_err(), "Truncated at {} should fail", cut);
    }
}

#[test]
fn test_geometry_invalid_root_chunk() {
    let mut cube_bytes = build_synthetic_cube_geometry();
    // Overwrite root container chunk ID
    cube_bytes[0] = 0xAA;
    cube_bytes[1] = 0xBB;
    cube_bytes[2] = 0xCC;
    cube_bytes[3] = 0xDD;

    let res = parse_geometry(&cube_bytes);
    assert!(matches!(res, Err(GeometryError::MissingMandatoryChunk(_))));
}

#[test]
fn test_geometry_oversized_chunk_length() {
    let mut cube_bytes = build_synthetic_cube_geometry();
    // Overwrite root chunk length with 0xFFFFFFFF
    cube_bytes[4] = 0xFF;
    cube_bytes[5] = 0xFF;
    cube_bytes[6] = 0xFF;
    cube_bytes[7] = 0xFF;

    let res = parse_geometry(&cube_bytes);
    assert!(matches!(
        res,
        Err(GeometryError::Chunk(ChunkError::InvalidChunkSize { .. }))
    ));
}

#[test]
fn test_jdlz_empty_and_truncated_headers() {
    assert!(matches!(
        decompress_jdlz(&[]),
        Err(TextureError::UnexpectedEof { .. })
    ));

    let small = [0u8; 15];
    assert!(matches!(
        decompress_jdlz(&small),
        Err(TextureError::UnexpectedEof { .. })
    ));
}

#[test]
fn test_jdlz_invalid_magic() {
    let mut header = [0u8; 16];
    header[0..4].copy_from_slice(b"BAD!");
    header[4] = 2; // version 2
    assert!(matches!(
        decompress_jdlz(&header),
        Err(TextureError::InvalidJdlzMagic(_))
    ));
}

#[test]
fn test_jdlz_unsupported_version() {
    let mut header = [0u8; 16];
    header[0..4].copy_from_slice(b"JDLZ");
    header[4] = 3; // version 3
    assert!(matches!(
        decompress_jdlz(&header),
        Err(TextureError::UnsupportedJdlzVersion(3))
    ));
}

#[test]
fn test_jdlz_invalid_compressed_size() {
    let mut header = [0u8; 16];
    header[0..4].copy_from_slice(b"JDLZ");
    header[4] = 2;
    // uncompressed size: 100
    header[8..12].copy_from_slice(&100u32.to_le_bytes());
    // compressed size: 5000 (exceeds buffer)
    header[12..16].copy_from_slice(&5000u32.to_le_bytes());

    assert!(matches!(
        decompress_jdlz(&header),
        Err(TextureError::UnexpectedEof { .. })
    ));
}

#[test]
fn test_jdlz_invalid_backreference_fails_gracefully() {
    // Construct a malicious stream that attempts to match-copy before any bytes have been written
    let mut stream = Vec::new();
    stream.extend_from_slice(b"JDLZ");
    stream.push(2); // version 2
    stream.extend_from_slice(&[0u8; 3]);
    stream.extend_from_slice(&100u32.to_le_bytes()); // uncompressed size = 100
    stream.extend_from_slice(&20u32.to_le_bytes()); // compressed size = 20

    // flags1 = 0x01 (match copy), flags2 = 0x00
    stream.push(0x01);
    stream.push(0x00);
    // match bytes: offset > 0 when out is empty
    stream.push(0x20);
    stream.push(0x05);

    let res = decompress_jdlz(&stream);
    assert!(
        matches!(res, Err(TextureError::InvalidBackReference { .. })),
        "Expected InvalidBackReference, got {:?}",
        res
    );
}

#[test]
fn test_texture_pack_empty_input() {
    let pack = parse_texture_pack(&[]).expect("Empty texture pack should return empty pack");
    assert!(pack.textures.is_empty());
}

#[test]
fn test_geometry_unaligned_slice_safety() {
    let synthetic = open_nfs_asset::synthetic::build_synthetic_cube_geometry();
    for pad in 1..=3 {
        let mut unaligned = vec![0xBB; pad];
        unaligned.extend_from_slice(&synthetic);

        let parsed = open_nfs_asset::geometry::parse_geometry(&unaligned[pad..])
            .expect("Should safely parse unaligned geometry slice without panic");
        assert_eq!(parsed.parts.len(), 1);
        assert_eq!(parsed.parts[0].vertices.len(), 8);
        assert_eq!(parsed.parts[0].indices.len(), 36);

        let err = open_nfs_asset::geometry::parse_geometry(&unaligned[..pad + 20]);
        assert!(err.is_err());
    }
}

#[test]
fn test_dxt_decoders_invalid_dimensions_and_truncated_data() {
    let mut out = [0u8; 64];

    // Zero dimensions should fail
    assert!(matches!(
        open_nfs_asset::texture::decode_dxt1(&[0u8; 8], 0, 4, &mut out),
        Err(TextureError::InvalidDimensions { .. })
    ));
    assert!(matches!(
        open_nfs_asset::texture::decode_dxt3(&[0u8; 16], 4, 0, &mut out),
        Err(TextureError::InvalidDimensions { .. })
    ));
    assert!(matches!(
        open_nfs_asset::texture::decode_dxt5(&[0u8; 16], 0, 0, &mut out),
        Err(TextureError::InvalidDimensions { .. })
    ));

    // Truncated data buffers
    assert!(matches!(
        open_nfs_asset::texture::decode_dxt1(&[0u8; 7], 4, 4, &mut out),
        Err(TextureError::UnexpectedEof { .. })
    ));
    assert!(matches!(
        open_nfs_asset::texture::decode_dxt3(&[0u8; 15], 4, 4, &mut out),
        Err(TextureError::UnexpectedEof { .. })
    ));
    assert!(matches!(
        open_nfs_asset::texture::decode_dxt5(&[0u8; 15], 4, 4, &mut out),
        Err(TextureError::UnexpectedEof { .. })
    ));
}

#[test]
fn test_dxt1_solid_color_synthetic_block() {
    // 4x4 DXT1 block with pure red (c0 = 0xF800, c1 = 0x0000, lookup = 0x00000000)
    let mut block = [0u8; 8];
    block[0..2].copy_from_slice(&0xF800u16.to_le_bytes());
    block[2..4].copy_from_slice(&0x0000u16.to_le_bytes());
    block[4..8].copy_from_slice(&0x00000000u32.to_le_bytes());

    let mut out = [0u8; 4 * 4 * 4];
    open_nfs_asset::texture::decode_dxt1(&block, 4, 4, &mut out)
        .expect("Failed to decode synthetic DXT1 block");

    // All pixels should be pure red (255, 0, 0, 255)
    for pixel in out.chunks_exact(4) {
        assert_eq!(pixel, &[255, 0, 0, 255]);
    }
}
