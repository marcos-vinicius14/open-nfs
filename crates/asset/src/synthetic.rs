//! Procedural synthetic fixture builder for testing without proprietary assets (ADR-008, NFR-009).

use crate::geometry::{
    SubMeshSlice, Vertex36, SOLID_CONTAINER, SOLID_INDEX_STREAM, SOLID_MESH_CONTAINER,
    SOLID_MESH_DESCRIPTOR, SOLID_PART_CONTAINER, SOLID_PART_HEADER, SOLID_TRIANGLES,
    SOLID_VERTEX_STREAM,
};
use open_nfs_core::types::ChunkId;

/// Constructs a fully compliant EA car geometry container byte buffer representing a synthetic test cube.
pub fn build_synthetic_cube_geometry() -> Vec<u8> {
    let vertices = vec![
        // Front face
        Vertex36 {
            position: [-1.0, -1.0, 1.0],
            normal: [0.0, 0.0, 1.0],
            color: 0xFF000000,
            uv: [0.0, 0.0],
        },
        Vertex36 {
            position: [1.0, -1.0, 1.0],
            normal: [0.0, 0.0, 1.0],
            color: 0xFF000000,
            uv: [1.0, 0.0],
        },
        Vertex36 {
            position: [1.0, 1.0, 1.0],
            normal: [0.0, 0.0, 1.0],
            color: 0xFF000000,
            uv: [1.0, 1.0],
        },
        Vertex36 {
            position: [-1.0, 1.0, 1.0],
            normal: [0.0, 0.0, 1.0],
            color: 0xFF000000,
            uv: [0.0, 1.0],
        },
        // Back face
        Vertex36 {
            position: [-1.0, -1.0, -1.0],
            normal: [0.0, 0.0, -1.0],
            color: 0xFF000000,
            uv: [1.0, 0.0],
        },
        Vertex36 {
            position: [-1.0, 1.0, -1.0],
            normal: [0.0, 0.0, -1.0],
            color: 0xFF000000,
            uv: [1.0, 1.0],
        },
        Vertex36 {
            position: [1.0, 1.0, -1.0],
            normal: [0.0, 0.0, -1.0],
            color: 0xFF000000,
            uv: [0.0, 1.0],
        },
        Vertex36 {
            position: [1.0, -1.0, -1.0],
            normal: [0.0, 0.0, -1.0],
            color: 0xFF000000,
            uv: [0.0, 0.0],
        },
    ];

    #[rustfmt::skip]
    let indices: Vec<u16> = vec![
        0, 1, 2,  2, 3, 0, // front
        1, 7, 6,  6, 2, 1, // right
        7, 4, 5,  5, 6, 7, // back
        4, 0, 3,  3, 5, 4, // left
        3, 2, 6,  6, 5, 3, // top
        4, 7, 1,  1, 0, 4, // bottom
    ];

    let submesh = SubMeshSlice {
        bbox_min: [-1.0, -1.0, -1.0],
        bbox_max: [1.0, 1.0, 1.0],
        index_count: 36,
        index_offset: 0,
        material_id: 1,
        shader_flags: 0x0000_4180,
    };

    build_custom_geometry("SYNTHETIC_CUBE_A", &vertices, &indices, &[submesh])
}

/// Helper function to build custom procedural geometry chunk bytes.
pub fn build_custom_geometry(
    part_name: &str,
    vertices: &[Vertex36],
    indices: &[u16],
    submeshes: &[SubMeshSlice],
) -> Vec<u8> {
    let mut part_payload = Vec::new();

    // 1. Part Header (192 bytes)
    let mut header_bytes = vec![0u8; 192];
    header_bytes[0x0E..0x10].copy_from_slice(&0u16.to_le_bytes()); // LOD high
    header_bytes[0x10..0x14].copy_from_slice(&0x1234_5678u32.to_le_bytes()); // part hash
    let tri_count = (indices.len() / 3) as u32;
    header_bytes[0x14..0x18].copy_from_slice(&tri_count.to_le_bytes());
    header_bytes[0x18..0x1A].copy_from_slice(&(vertices.len() as u16).to_le_bytes());
    header_bytes[0x1A] = submeshes.len() as u8;

    header_bytes[0x20..0x24].copy_from_slice(&(-1.0f32).to_le_bytes());
    header_bytes[0x24..0x28].copy_from_slice(&(-1.0f32).to_le_bytes());
    header_bytes[0x28..0x2C].copy_from_slice(&(-1.0f32).to_le_bytes());

    header_bytes[0x30..0x34].copy_from_slice(&(1.0f32).to_le_bytes());
    header_bytes[0x34..0x38].copy_from_slice(&(1.0f32).to_le_bytes());
    header_bytes[0x38..0x3C].copy_from_slice(&(1.0f32).to_le_bytes());

    let name_bytes = part_name.as_bytes();
    let name_len = name_bytes.len().min(27);
    header_bytes[0xA4..0xA4 + name_len].copy_from_slice(&name_bytes[..name_len]);

    append_chunk(&mut part_payload, SOLID_PART_HEADER, &header_bytes);

    // 2. Mesh Container
    let mut mesh_payload = Vec::new();

    // 2.1 Mesh Descriptor (68 bytes)
    let mut desc_bytes = vec![0u8; 68];
    desc_bytes[0x08..0x0C].copy_from_slice(&0x10u32.to_le_bytes());
    desc_bytes[0x0C..0x10].copy_from_slice(&0xC180u32.to_le_bytes());
    desc_bytes[0x10..0x14].copy_from_slice(&(submeshes.len() as u32).to_le_bytes());
    desc_bytes[0x24..0x28].copy_from_slice(&tri_count.to_le_bytes());
    desc_bytes[0x34..0x38].copy_from_slice(&(vertices.len() as u32).to_le_bytes());
    append_chunk(&mut mesh_payload, SOLID_MESH_DESCRIPTOR, &desc_bytes);

    // 2.2 Vertex Stream with 128-byte alignment
    // Root header (8) + Part container header (8) + Part header chunk (8 + 192) + Mesh container header (8) = 224
    const BASE_OFFSET: usize = 224;

    let chunk_file_offset = BASE_OFFSET + mesh_payload.len();
    let target_aligned = (chunk_file_offset + 0x87) & !0x7F;
    let padding_len = target_aligned.saturating_sub(chunk_file_offset + 8);
    let mut vert_chunk_payload = vec![0x11u8; padding_len];
    let vert_raw: &[u8] = bytemuck::cast_slice(vertices);
    vert_chunk_payload.extend_from_slice(vert_raw);
    append_chunk(&mut mesh_payload, SOLID_VERTEX_STREAM, &vert_chunk_payload);

    // 2.3 Submesh table (16-byte alignment, 60 bytes each)
    let chunk_file_offset = BASE_OFFSET + mesh_payload.len();
    let target_aligned = (chunk_file_offset + 0x17) & !0xF;
    let padding_len = target_aligned.saturating_sub(chunk_file_offset + 8);
    let mut sm_chunk_payload = vec![0x11u8; padding_len];
    for sm in submeshes {
        let mut entry = vec![0u8; 60];
        entry[0..4].copy_from_slice(&sm.bbox_min[0].to_le_bytes());
        entry[4..8].copy_from_slice(&sm.bbox_min[1].to_le_bytes());
        entry[8..12].copy_from_slice(&sm.bbox_min[2].to_le_bytes());
        entry[12..16].copy_from_slice(&sm.index_count.to_le_bytes());
        entry[16..20].copy_from_slice(&sm.bbox_max[0].to_le_bytes());
        entry[20..24].copy_from_slice(&sm.bbox_max[1].to_le_bytes());
        entry[24..28].copy_from_slice(&sm.bbox_max[2].to_le_bytes());
        entry[28..32].copy_from_slice(&sm.material_id.to_le_bytes());
        entry[52..56].copy_from_slice(&sm.index_offset.to_le_bytes());
        entry[56..60].copy_from_slice(&sm.shader_flags.to_le_bytes());
        sm_chunk_payload.extend_from_slice(&entry);
    }
    append_chunk(&mut mesh_payload, SOLID_INDEX_STREAM, &sm_chunk_payload);

    // 2.4 Triangle indices (16-byte alignment, u16 each)
    let chunk_file_offset = BASE_OFFSET + mesh_payload.len();
    let target_aligned = (chunk_file_offset + 0x17) & !0xF;
    let padding_len = target_aligned.saturating_sub(chunk_file_offset + 8);
    let mut idx_chunk_payload = vec![0x11u8; padding_len];
    let idx_raw: &[u8] = bytemuck::cast_slice(indices);
    idx_chunk_payload.extend_from_slice(idx_raw);
    append_chunk(&mut mesh_payload, SOLID_TRIANGLES, &idx_chunk_payload);

    append_chunk(&mut part_payload, SOLID_MESH_CONTAINER, &mesh_payload);

    // 3. Root Solid Container
    let mut root_payload = Vec::new();
    append_chunk(&mut root_payload, SOLID_PART_CONTAINER, &part_payload);

    let mut final_data = Vec::new();
    append_chunk(&mut final_data, SOLID_CONTAINER, &root_payload);
    final_data
}

fn append_chunk(buffer: &mut Vec<u8>, id: ChunkId, payload: &[u8]) {
    buffer.extend_from_slice(&id.raw().to_le_bytes());
    buffer.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    buffer.extend_from_slice(payload);
}
