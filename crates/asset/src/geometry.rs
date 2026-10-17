//! Bounded, safe parser for EA vehicle geometry containers (`GEOMETRY.BIN`).

use crate::chunk::ChunkIterator;
use crate::error::GeometryError;
use bytemuck::Zeroable;
use open_nfs_core::types::ChunkId;

/// Known chunk IDs for EA car geometry archives.
pub const SOLID_CONTAINER: ChunkId = ChunkId(0x8013_4000);
pub const SOLID_HEADER_CONTAINER: ChunkId = ChunkId(0x8013_4001);
pub const SOLID_PART_CONTAINER: ChunkId = ChunkId(0x8013_4010);
pub const SOLID_PART_HEADER: ChunkId = ChunkId(0x0013_4011);
pub const SOLID_PART_TRANSFORM: ChunkId = ChunkId(0x0013_4012);
pub const SOLID_PART_MOUNTS: ChunkId = ChunkId(0x0013_4013);
pub const SOLID_MESH_CONTAINER: ChunkId = ChunkId(0x8013_4100);
pub const SOLID_MESH_DESCRIPTOR: ChunkId = ChunkId(0x0013_4900);
pub const SOLID_VERTEX_STREAM: ChunkId = ChunkId(0x0013_4B01);
pub const SOLID_INDEX_STREAM: ChunkId = ChunkId(0x0013_4B02);
pub const SOLID_TRIANGLES: ChunkId = ChunkId(0x0013_4B03);

pub use open_nfs_core::math::RuntimeVertex;

/// Standard 36-byte vertex structure matching Direct3D FVF layout:
/// `D3DFVF_XYZ | D3DFVF_NORMAL | D3DFVF_DIFFUSE | D3DFVF_TEX1`.
/// Aliases [`RuntimeVertex`].
pub type Vertex36 = RuntimeVertex;

#[derive(Debug, Clone, PartialEq)]
pub struct SubMeshSlice {
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
    pub index_count: u32,
    pub index_offset: u32,
    pub material_id: u32,
    pub shader_flags: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PartHeader {
    pub name: String,
    pub lod_level: u16,
    pub part_hash: u32,
    pub triangle_count: u32,
    pub vertex_count: u16,
    pub submesh_count: u8,
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedCarPart {
    pub header: PartHeader,
    pub vertices: Vec<Vertex36>,
    pub indices: Vec<u16>,
    pub submeshes: Vec<SubMeshSlice>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParsedGeometryArchive {
    pub parts: Vec<ParsedCarPart>,
}

impl ParsedGeometryArchive {
    pub fn find_part(&self, name: &str) -> Option<&ParsedCarPart> {
        self.parts.iter().find(|p| p.header.name == name)
    }

    pub fn base_part(&self) -> Option<&ParsedCarPart> {
        self.parts
            .iter()
            .find(|p| p.header.name.ends_with("_BASE_A"))
    }
}

pub fn parse_geometry(buffer: &[u8]) -> Result<ParsedGeometryArchive, GeometryError> {
    if buffer.is_empty() {
        return Err(GeometryError::Chunk(
            crate::error::ChunkError::UnexpectedEof {
                offset: 0,
                expected: 8,
                available: 0,
            },
        ));
    }

    let mut iter = ChunkIterator::new(buffer, 0);
    let mut archive = ParsedGeometryArchive::default();
    let mut found_solid_container = false;

    while let Some(top_chunk) = iter.next_chunk()? {
        if top_chunk.id == SOLID_CONTAINER {
            found_solid_container = true;
            let mut sub_iter = top_chunk.iter_subchunks();
            while let Some(sub_chunk) = sub_iter.next_chunk()? {
                if sub_chunk.id == SOLID_PART_CONTAINER {
                    let part = parse_part_container(sub_chunk.payload, sub_chunk.offset + 8)?;
                    archive.parts.push(part);
                }
            }
        }
    }

    if !found_solid_container {
        return Err(GeometryError::MissingMandatoryChunk(SOLID_CONTAINER));
    }

    Ok(archive)
}

fn parse_part_container(
    payload: &[u8],
    base_offset: usize,
) -> Result<ParsedCarPart, GeometryError> {
    let mut iter = ChunkIterator::new(payload, base_offset);
    let mut header_opt: Option<PartHeader> = None;
    let mut vertices: Vec<Vertex36> = Vec::new();
    let mut indices: Vec<u16> = Vec::new();
    let mut submeshes: Vec<SubMeshSlice> = Vec::new();

    while let Some(chunk) = iter.next_chunk()? {
        match chunk.id {
            SOLID_PART_HEADER => {
                header_opt = Some(parse_part_header(chunk.payload, chunk.offset)?);
            }
            SOLID_MESH_CONTAINER => {
                let streams = parse_mesh_container(chunk.payload, chunk.offset + 8)?;
                vertices = streams.vertices;
                indices = streams.indices;
                submeshes = streams.submeshes;
            }
            _ => {
                // Ignore auxiliary chunks (0x00134012 transforms, 0x00134013 mounts, animations)
            }
        }
    }

    let header = header_opt.ok_or(GeometryError::MissingMandatoryChunk(SOLID_PART_HEADER))?;

    let vert_count = vertices.len() as u32;
    for &idx in &indices {
        if idx as u32 >= vert_count {
            return Err(GeometryError::IndexOutOfBounds {
                index: idx,
                vertex_count: vert_count,
            });
        }
    }

    Ok(ParsedCarPart {
        header,
        vertices,
        indices,
        submeshes,
    })
}

/// Parse the 192-byte `SOLID_PART_HEADER` (`0x00134011`).
fn parse_part_header(data: &[u8], offset: usize) -> Result<PartHeader, GeometryError> {
    if data.len() < 192 {
        return Err(GeometryError::Chunk(
            crate::error::ChunkError::UnexpectedEof {
                offset,
                expected: 192,
                available: data.len(),
            },
        ));
    }

    let lod_level = u16::from_le_bytes([data[0x0E], data[0x0F]]);
    let part_hash = u32::from_le_bytes([data[0x10], data[0x11], data[0x12], data[0x13]]);
    let triangle_count = u32::from_le_bytes([data[0x14], data[0x15], data[0x16], data[0x17]]);
    let vertex_count = u16::from_le_bytes([data[0x18], data[0x19]]);
    let submesh_count = data[0x1A];

    let bbox_min = [
        f32::from_le_bytes([data[0x20], data[0x21], data[0x22], data[0x23]]),
        f32::from_le_bytes([data[0x24], data[0x25], data[0x26], data[0x27]]),
        f32::from_le_bytes([data[0x28], data[0x29], data[0x2A], data[0x2B]]),
    ];

    let bbox_max = [
        f32::from_le_bytes([data[0x30], data[0x31], data[0x32], data[0x33]]),
        f32::from_le_bytes([data[0x34], data[0x35], data[0x36], data[0x37]]),
        f32::from_le_bytes([data[0x38], data[0x39], data[0x3A], data[0x3B]]),
    ];

    // Read null-terminated ASCII string from 0xA4..0xC0 (28 bytes)
    let name_bytes = &data[0xA4..0xC0];
    let len = name_bytes
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(name_bytes.len());
    let name = std::str::from_utf8(&name_bytes[..len])
        .map_err(|_| GeometryError::InvalidPartName("non-utf8 characters in part identifier"))?
        .to_string();

    Ok(PartHeader {
        name,
        lod_level,
        part_hash,
        triangle_count,
        vertex_count,
        submesh_count,
        bbox_min,
        bbox_max,
    })
}

#[derive(Debug, Default)]
struct ParsedMeshStreams {
    vertices: Vec<Vertex36>,
    indices: Vec<u16>,
    submeshes: Vec<SubMeshSlice>,
}

/// Parse `SOLID_MESH_CONTAINER` (`0x80134100`).
fn parse_mesh_container(
    payload: &[u8],
    base_offset: usize,
) -> Result<ParsedMeshStreams, GeometryError> {
    let mut iter = ChunkIterator::new(payload, base_offset);
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut submeshes = Vec::new();

    let mut desc_vcount: usize = 0;

    while let Some(chunk) = iter.next_chunk()? {
        match chunk.id {
            SOLID_MESH_DESCRIPTOR => {
                if chunk.payload.len() >= 0x38 {
                    desc_vcount = u32::from_le_bytes([
                        chunk.payload[0x34],
                        chunk.payload[0x35],
                        chunk.payload[0x36],
                        chunk.payload[0x37],
                    ]) as usize;
                }
            }
            SOLID_VERTEX_STREAM => {
                // Alignment: (chunk.offset + 0x87) & !0x7F
                // Relative to chunk.payload:
                let header_start = chunk.offset;
                let aligned_start = (header_start + 0x87) & !0x7F;
                let align_padding = aligned_start.saturating_sub(header_start + 8);

                if align_padding > chunk.payload.len() {
                    return Err(GeometryError::InvalidVertexStride { payload_bytes: 0 });
                }

                let vert_bytes = &chunk.payload[align_padding..];
                if desc_vcount > 0 && desc_vcount * 24 == vert_bytes.len() {
                    // 24-byte unlit vertex stride (e.g. engine bays, unlit undercarriage):
                    // pos: [f32; 3] (12 bytes), color: u32 (4 bytes), uv: [f32; 2] (8 bytes)
                    vertices = Vec::with_capacity(desc_vcount);
                    for i in 0..desc_vcount {
                        let entry = &vert_bytes[i * 24..(i + 1) * 24];
                        let position = [
                            f32::from_le_bytes([entry[0], entry[1], entry[2], entry[3]]),
                            f32::from_le_bytes([entry[4], entry[5], entry[6], entry[7]]),
                            f32::from_le_bytes([entry[8], entry[9], entry[10], entry[11]]),
                        ];
                        let color =
                            u32::from_le_bytes([entry[12], entry[13], entry[14], entry[15]]);
                        let uv = [
                            f32::from_le_bytes([entry[16], entry[17], entry[18], entry[19]]),
                            f32::from_le_bytes([entry[20], entry[21], entry[22], entry[23]]),
                        ];
                        vertices.push(Vertex36 {
                            position,
                            normal: [0.0, 1.0, 0.0],
                            color,
                            uv,
                        });
                    }
                } else if vert_bytes.len() % std::mem::size_of::<Vertex36>() == 0 {
                    let count = vert_bytes.len() / std::mem::size_of::<Vertex36>();
                    if let Ok(casted) = bytemuck::try_cast_slice::<u8, Vertex36>(vert_bytes) {
                        vertices = casted.to_vec();
                    } else {
                        vertices = vec![Vertex36::zeroed(); count];
                        bytemuck::cast_slice_mut(&mut vertices).copy_from_slice(vert_bytes);
                    }
                } else {
                    return Err(GeometryError::InvalidVertexStride {
                        payload_bytes: vert_bytes.len(),
                    });
                }
            }
            SOLID_INDEX_STREAM => {
                // Submesh slices: 60 bytes each
                // Alignment: (chunk.offset + 0x17) & !0xF
                let header_start = chunk.offset;
                let aligned_start = (header_start + 0x17) & !0xF;
                let align_padding = aligned_start.saturating_sub(header_start + 8);

                if align_padding < chunk.payload.len() {
                    let slice_bytes = &chunk.payload[align_padding..];
                    let count = slice_bytes.len() / 60;
                    for i in 0..count {
                        let entry = &slice_bytes[i * 60..(i + 1) * 60];
                        let bbox_min = [
                            f32::from_le_bytes([entry[0], entry[1], entry[2], entry[3]]),
                            f32::from_le_bytes([entry[4], entry[5], entry[6], entry[7]]),
                            f32::from_le_bytes([entry[8], entry[9], entry[10], entry[11]]),
                        ];
                        let index_count =
                            u32::from_le_bytes([entry[12], entry[13], entry[14], entry[15]]);
                        let bbox_max = [
                            f32::from_le_bytes([entry[16], entry[17], entry[18], entry[19]]),
                            f32::from_le_bytes([entry[20], entry[21], entry[22], entry[23]]),
                            f32::from_le_bytes([entry[24], entry[25], entry[26], entry[27]]),
                        ];
                        let material_id =
                            u32::from_le_bytes([entry[28], entry[29], entry[30], entry[31]]);
                        let index_offset =
                            u32::from_le_bytes([entry[52], entry[53], entry[54], entry[55]]);
                        let shader_flags =
                            u32::from_le_bytes([entry[56], entry[57], entry[58], entry[59]]);

                        submeshes.push(SubMeshSlice {
                            bbox_min,
                            bbox_max,
                            index_count,
                            index_offset,
                            material_id,
                            shader_flags,
                        });
                    }
                }
            }
            SOLID_TRIANGLES => {
                // Alignment: (chunk.offset + 0x17) & !0xF
                let header_start = chunk.offset;
                let aligned_start = (header_start + 0x17) & !0xF;
                let align_padding = aligned_start.saturating_sub(header_start + 8);

                if align_padding < chunk.payload.len() {
                    let idx_bytes = &chunk.payload[align_padding..];
                    // Truncate to multiple of 2
                    let valid_len = (idx_bytes.len() / 2) * 2;
                    let count = valid_len / 2;
                    if let Ok(casted) = bytemuck::try_cast_slice::<u8, u16>(&idx_bytes[..valid_len])
                    {
                        indices = casted.to_vec();
                    } else {
                        indices = vec![0u16; count];
                        bytemuck::cast_slice_mut(&mut indices)
                            .copy_from_slice(&idx_bytes[..valid_len]);
                    }
                }
            }
            _ => {}
        }
    }

    Ok(ParsedMeshStreams {
        vertices,
        indices,
        submeshes,
    })
}
