//! Bounded, safe parser for EA texture packages (`TEXTURES.BIN`) and JDLZ decompression.

use crate::chunk::ChunkIterator;
use crate::error::TextureError;
use open_nfs_core::types::ChunkId;

pub const TEXTURE_PACK_CONTAINER: ChunkId = ChunkId(0xB330_0000);
pub const TEXTURE_INFO_CONTAINER: ChunkId = ChunkId(0xB331_0000);
pub const TEXTURE_PACK_HEADER: ChunkId = ChunkId(0x3331_0001);
pub const TEXTURE_HASH_TABLE: ChunkId = ChunkId(0x3331_0002);
pub const TEXTURE_METADATA_TABLE: ChunkId = ChunkId(0x3331_0003);
pub const TEXTURE_DATA_CONTAINER: ChunkId = ChunkId(0xB332_0000);
pub const TEXTURE_DATA_HEADER: ChunkId = ChunkId(0x3332_0001);
pub const TEXTURE_DATA_STREAM: ChunkId = ChunkId(0x3332_0002);

const MAX_SAFE_UNCOMPRESSED_TEXTURE_BYTES: usize = 64 * 1024 * 1024; // 64 MiB sanity limit

/// Recognized pixel / compression formats used in NFSU2 textures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureFormat {
    Dxt1,
    Dxt3,
    Dxt5,
    Rgba32,
    Unknown(u8),
}

/// Metadata and pixel data for an individual texture map within a texture pack.
#[derive(Debug, Clone, PartialEq)]
pub struct TextureEntry {
    pub hash: u32,
    pub width: u16,
    pub height: u16,
    pub mip_levels: u8,
    pub format: TextureFormat,
    pub data: Vec<u8>,
}

/// A collection of textures extracted from a `TEXTURES.BIN` archive.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TexturePack {
    pub package_name: String,
    pub textures: Vec<TextureEntry>,
}

/// Decompress an EA JDLZ (version 2 LZSS) compressed byte stream.
///
/// Traced directly from `SPEED2.EXE` function `func_0x00573650`.
pub fn decompress_jdlz(data: &[u8]) -> Result<Vec<u8>, TextureError> {
    if data.len() < 16 {
        return Err(TextureError::UnexpectedEof {
            offset: 0,
            expected: 16,
            available: data.len(),
        });
    }

    let magic: [u8; 4] = [data[0], data[1], data[2], data[3]];
    if &magic != b"JDLZ" {
        return Err(TextureError::InvalidJdlzMagic(magic));
    }

    let version = data[4];
    if version != 2 {
        return Err(TextureError::UnsupportedJdlzVersion(version));
    }

    let uncompressed_size = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
    let compressed_size = u32::from_le_bytes([data[12], data[13], data[14], data[15]]) as usize;

    if uncompressed_size > MAX_SAFE_UNCOMPRESSED_TEXTURE_BYTES {
        return Err(TextureError::DecompressionOverflow {
            expected: MAX_SAFE_UNCOMPRESSED_TEXTURE_BYTES,
            actual: uncompressed_size,
        });
    }

    if compressed_size > data.len() {
        return Err(TextureError::UnexpectedEof {
            offset: 0,
            expected: compressed_size,
            available: data.len(),
        });
    }

    let mut out = Vec::with_capacity(uncompressed_size);
    let mut in_idx = 16;
    let mut flags1 = (data[in_idx] as u16) | 0x100;
    in_idx += 1;
    let mut flags2 = (data[in_idx] as u16) | 0x100;
    in_idx += 1;

    let mut bytes_left = compressed_size.saturating_sub(18);

    while bytes_left > 0 && out.len() < uncompressed_size {
        if (flags1 & 1) != 0 {
            // Match copy
            if in_idx + 2 > compressed_size {
                return Err(TextureError::UnexpectedEof {
                    offset: in_idx,
                    expected: 2,
                    available: compressed_size.saturating_sub(in_idx),
                });
            }

            let (length, offset) = if (flags2 & 1) != 0 {
                let b1 = data[in_idx] as usize;
                let b2 = data[in_idx + 1] as usize;
                in_idx += 2;
                bytes_left = bytes_left.saturating_sub(2);
                let length = (((b1 & 0xF0) << 4) | b2) + 3;
                let offset = (b1 & 0x0F) + 1;
                (length, offset)
            } else {
                let b1 = data[in_idx] as usize;
                let b2 = data[in_idx + 1] as usize;
                in_idx += 2;
                bytes_left = bytes_left.saturating_sub(2);
                let offset = (((b1 & 0xE0) << 3) | b2) + 17;
                let length = (b1 & 0x1F) + 3;
                (length, offset)
            };

            flags2 >>= 1;

            if offset > out.len() {
                return Err(TextureError::InvalidBackReference {
                    offset,
                    current_len: out.len(),
                });
            }

            for _ in 0..length {
                if out.len() >= uncompressed_size {
                    break;
                }
                let src_pos = out.len() - offset;
                let byte = out[src_pos];
                out.push(byte);
            }
        } else {
            // Literal copy
            if in_idx >= compressed_size {
                return Err(TextureError::UnexpectedEof {
                    offset: in_idx,
                    expected: 1,
                    available: 0,
                });
            }
            out.push(data[in_idx]);
            in_idx += 1;
            bytes_left = bytes_left.saturating_sub(1);
        }

        flags1 >>= 1;

        if flags1 == 1 && bytes_left > 0 && in_idx < compressed_size {
            flags1 = (data[in_idx] as u16) | 0x100;
            in_idx += 1;
            bytes_left = bytes_left.saturating_sub(1);
        }

        if flags2 == 1 && bytes_left > 0 && in_idx < compressed_size {
            flags2 = (data[in_idx] as u16) | 0x100;
            in_idx += 1;
            bytes_left = bytes_left.saturating_sub(1);
        }
    }

    Ok(out)
}

/// Parse a texture pack archive from memory.
pub fn parse_texture_pack(buffer: &[u8]) -> Result<TexturePack, TextureError> {
    let mut iter = ChunkIterator::new(buffer, 0);
    let mut pack = TexturePack::default();

    while let Some(top_chunk) = iter.next_chunk()? {
        if top_chunk.id == TEXTURE_PACK_CONTAINER {
            let mut sub_iter = top_chunk.iter_subchunks();
            while let Some(sub_chunk) = sub_iter.next_chunk()? {
                if sub_chunk.id == TEXTURE_INFO_CONTAINER {
                    parse_texture_info(sub_chunk.payload, &mut pack)?;
                }
            }
        }
    }

    Ok(pack)
}

fn parse_texture_info(payload: &[u8], pack: &mut TexturePack) -> Result<(), TextureError> {
    let mut iter = ChunkIterator::new(payload, 0);
    while let Some(chunk) = iter.next_chunk()? {
        if chunk.id == TEXTURE_PACK_HEADER && chunk.payload.len() >= 64 {
            let name_bytes = &chunk.payload[0x20..0x60];
            let len = name_bytes
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(name_bytes.len());
            if let Ok(name) = std::str::from_utf8(&name_bytes[..len]) {
                pack.package_name = name.to_string();
            }
        } else if chunk.id == TEXTURE_METADATA_TABLE {
            let entry_size = 24;
            let count = chunk.payload.len() / entry_size;
            for i in 0..count {
                let entry_data = &chunk.payload[i * entry_size..(i + 1) * entry_size];
                let hash = u32::from_le_bytes([
                    entry_data[0],
                    entry_data[1],
                    entry_data[2],
                    entry_data[3],
                ]);
                let width = u16::from_le_bytes([entry_data[16], entry_data[17]]);
                let height = u16::from_le_bytes([entry_data[18], entry_data[19]]);
                let mip_levels = entry_data[20];
                let fmt_byte = entry_data[21];

                let format = match fmt_byte {
                    0x27 => TextureFormat::Dxt1,
                    0x29 => TextureFormat::Dxt3,
                    0x2A => TextureFormat::Dxt5,
                    0x28 => TextureFormat::Rgba32,
                    other => TextureFormat::Unknown(other),
                };

                pack.textures.push(TextureEntry {
                    hash,
                    width,
                    height,
                    mip_levels,
                    format,
                    data: Vec::new(),
                });
            }
        }
    }
    Ok(())
}
