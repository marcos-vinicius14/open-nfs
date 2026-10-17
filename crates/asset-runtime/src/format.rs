//! Serialization and safe deserialization for `.onfs-model` (v1) binary files.

use crate::error::RuntimeAssetError;
use crate::model::{RuntimeBounds, RuntimeModel, RuntimePart, RuntimeSubmesh, RuntimeVertex};
use bytemuck::Zeroable;

pub const ONFM_MAGIC: [u8; 4] = *b"ONFM";
pub const ONFM_VERSION: u32 = 1;
pub const HEADER_SIZE: usize = 64;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct OnfmHeader {
    pub magic: [u8; 4],
    pub version: u32,
    pub source_sha256: [u8; 32],
    pub part_count: u32,
    pub total_vertices: u32,
    pub total_indices: u32,
    pub _padding: [u8; 12],
}

/// Serialize an in-memory `RuntimeModel` into `.onfs-model` v1 binary bytes.
pub fn serialize_runtime_model(model: &RuntimeModel) -> Vec<u8> {
    let mut out = Vec::new();

    // 1. Header (64 bytes)
    let header = OnfmHeader {
        magic: ONFM_MAGIC,
        version: ONFM_VERSION,
        source_sha256: model.source_sha256,
        part_count: model.parts.len() as u32,
        total_vertices: model.total_vertices() as u32,
        total_indices: model.total_indices() as u32,
        _padding: [0u8; 12],
    };
    out.extend_from_slice(bytemuck::bytes_of(&header));

    // 2. Parts
    for part in &model.parts {
        let name_bytes = part.name.as_bytes();
        out.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        out.extend_from_slice(name_bytes);

        out.extend_from_slice(&part.lod_level.to_le_bytes());
        out.extend_from_slice(&part.part_hash.to_le_bytes());

        // Bounds
        for c in part.bounds.min {
            out.extend_from_slice(&c.to_le_bytes());
        }
        for c in part.bounds.max {
            out.extend_from_slice(&c.to_le_bytes());
        }

        out.extend_from_slice(&(part.vertices.len() as u32).to_le_bytes());
        out.extend_from_slice(&(part.indices.len() as u32).to_le_bytes());
        out.extend_from_slice(&(part.submeshes.len() as u32).to_le_bytes());

        // Vertex buffer
        let v_bytes: &[u8] = bytemuck::cast_slice(&part.vertices);
        out.extend_from_slice(v_bytes);

        // Index buffer
        let i_bytes: &[u8] = bytemuck::cast_slice(&part.indices);
        out.extend_from_slice(i_bytes);

        // Submesh buffer
        let s_bytes: &[u8] = bytemuck::cast_slice(&part.submeshes);
        out.extend_from_slice(s_bytes);
    }

    out
}

/// Deserialize and validate a `.onfs-model` v1 binary buffer.
///
/// Guaranteed panic-free with strict bounds validation and index integrity checks.
pub fn deserialize_runtime_model(data: &[u8]) -> Result<RuntimeModel, RuntimeAssetError> {
    if data.len() < HEADER_SIZE {
        return Err(RuntimeAssetError::UnexpectedEof {
            offset: 0,
            expected: HEADER_SIZE,
            available: data.len(),
        });
    }

    let mut header = OnfmHeader::zeroed();
    bytemuck::bytes_of_mut(&mut header).copy_from_slice(&data[..HEADER_SIZE]);
    if header.magic != ONFM_MAGIC {
        return Err(RuntimeAssetError::InvalidMagic(header.magic));
    }
    if header.version != ONFM_VERSION {
        return Err(RuntimeAssetError::UnsupportedVersion {
            expected: ONFM_VERSION,
            found: header.version,
        });
    }

    let source_sha256 = header.source_sha256;
    let part_count = header.part_count as usize;

    let mut offset = HEADER_SIZE;
    let mut parts = Vec::with_capacity(part_count);

    for _ in 0..part_count {
        if offset + 2 > data.len() {
            return Err(RuntimeAssetError::UnexpectedEof {
                offset,
                expected: 2,
                available: data.len().saturating_sub(offset),
            });
        }
        let name_len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
        offset += 2;

        if offset + name_len > data.len() {
            return Err(RuntimeAssetError::UnexpectedEof {
                offset,
                expected: name_len,
                available: data.len().saturating_sub(offset),
            });
        }
        let name_str = std::str::from_utf8(&data[offset..offset + name_len])
            .map_err(|e| RuntimeAssetError::InvalidPartName(e.to_string()))?
            .to_string();
        offset += name_len;

        // Part metadata: lod_level (2), part_hash (4), bounds_min (12), bounds_max (12), vertex_count (4), index_count (4), submesh_count (4) = 42 bytes
        const PART_META_SIZE: usize = 2 + 4 + 12 + 12 + 4 + 4 + 4;
        if offset + PART_META_SIZE > data.len() {
            return Err(RuntimeAssetError::UnexpectedEof {
                offset,
                expected: PART_META_SIZE,
                available: data.len().saturating_sub(offset),
            });
        }

        let lod_level = u16::from_le_bytes([data[offset], data[offset + 1]]);
        offset += 2;

        let part_hash = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);
        offset += 4;

        let mut bounds = RuntimeBounds::default();
        for i in 0..3 {
            bounds.min[i] = f32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            offset += 4;
        }
        for i in 0..3 {
            bounds.max[i] = f32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            offset += 4;
        }

        let vertex_count = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);
        offset += 4;

        let index_count = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);
        offset += 4;

        let submesh_count = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);
        offset += 4;

        // Vertices (36 bytes each)
        let v_bytes_len = (vertex_count as usize)
            .checked_mul(std::mem::size_of::<RuntimeVertex>())
            .ok_or(RuntimeAssetError::IntegerOverflow)?;

        if offset + v_bytes_len > data.len() {
            return Err(RuntimeAssetError::UnexpectedEof {
                offset,
                expected: v_bytes_len,
                available: data.len().saturating_sub(offset),
            });
        }

        let mut vertices = vec![RuntimeVertex::zeroed(); vertex_count as usize];
        let v_dest: &mut [u8] = bytemuck::cast_slice_mut(&mut vertices);
        v_dest.copy_from_slice(&data[offset..offset + v_bytes_len]);
        offset += v_bytes_len;

        // Indices (2 bytes each)
        let i_bytes_len = (index_count as usize)
            .checked_mul(std::mem::size_of::<u16>())
            .ok_or(RuntimeAssetError::IntegerOverflow)?;

        if offset + i_bytes_len > data.len() {
            return Err(RuntimeAssetError::UnexpectedEof {
                offset,
                expected: i_bytes_len,
                available: data.len().saturating_sub(offset),
            });
        }

        let mut indices = vec![0u16; index_count as usize];
        let i_dest: &mut [u8] = bytemuck::cast_slice_mut(&mut indices);
        i_dest.copy_from_slice(&data[offset..offset + i_bytes_len]);
        offset += i_bytes_len;

        // Validate index stream references
        for &idx in &indices {
            if idx as u32 >= vertex_count {
                return Err(RuntimeAssetError::IndexOutOfBounds {
                    index: idx,
                    vertex_count,
                });
            }
        }

        // Submeshes (40 bytes each)
        let s_bytes_len = (submesh_count as usize)
            .checked_mul(std::mem::size_of::<RuntimeSubmesh>())
            .ok_or(RuntimeAssetError::IntegerOverflow)?;

        if offset + s_bytes_len > data.len() {
            return Err(RuntimeAssetError::UnexpectedEof {
                offset,
                expected: s_bytes_len,
                available: data.len().saturating_sub(offset),
            });
        }

        let mut submeshes = vec![RuntimeSubmesh::zeroed(); submesh_count as usize];
        let s_dest: &mut [u8] = bytemuck::cast_slice_mut(&mut submeshes);
        s_dest.copy_from_slice(&data[offset..offset + s_bytes_len]);
        offset += s_bytes_len;

        // Validate submesh index offsets and counts
        for sm in &submeshes {
            let end_index = sm
                .index_offset
                .checked_add(sm.index_count)
                .ok_or(RuntimeAssetError::IntegerOverflow)?;
            if end_index > index_count {
                return Err(RuntimeAssetError::UnexpectedEof {
                    offset: sm.index_offset as usize,
                    expected: sm.index_count as usize,
                    available: (index_count - sm.index_offset.min(index_count)) as usize,
                });
            }
        }

        parts.push(RuntimePart {
            name: name_str,
            lod_level,
            part_hash,
            bounds,
            vertices,
            indices,
            submeshes,
        });
    }

    Ok(RuntimeModel {
        source_sha256,
        parts,
    })
}
