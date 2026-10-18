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
    if buffer.is_empty() {
        return Ok(TexturePack::default());
    }

    let mut iter = ChunkIterator::new(buffer, 0);
    let mut pack = TexturePack::default();
    let mut raw_descs = Vec::new();

    while let Some(top_chunk) = iter.next_chunk()? {
        if top_chunk.id == TEXTURE_PACK_CONTAINER {
            let mut sub_iter = top_chunk.iter_subchunks();
            while let Some(sub_chunk) = sub_iter.next_chunk()? {
                if sub_chunk.id == TEXTURE_INFO_CONTAINER {
                    parse_texture_info(sub_chunk.payload, &mut pack, &mut raw_descs)?;
                }
            }
        }
    }

    // Extract texture data from buffer based on raw descriptors
    for desc in raw_descs {
        if desc
            .offset
            .checked_add(desc.comp_size)
            .is_none_or(|end| end > buffer.len())
        {
            continue;
        }

        let slice = &buffer[desc.offset..desc.offset + desc.comp_size];
        let decomp = if slice.len() >= 4 && &slice[..4] == b"JDLZ" {
            match decompress_jdlz(slice) {
                Ok(bytes) => bytes,
                Err(e) => {
                    tracing::warn!(
                        ?e,
                        hash = desc.hash,
                        "Failed to decompress JDLZ texture, skipping"
                    );
                    continue;
                }
            }
        } else if slice.len() >= 4
            && (&slice[..4] == b"HUFF" || &slice[..4] == b"COMP" || &slice[..4] == b"RAWW")
        {
            // Skip non-JDLZ compressed streams rather than misinterpreting compressed bytes as raw pixels
            continue;
        } else {
            slice.to_vec()
        };

        if decomp.len() >= 120 {
            let footer = &decomp[decomp.len() - 120..];
            let width = u16::from_le_bytes([footer[32], footer[33]]);
            let height = u16::from_le_bytes([footer[34], footer[35]]);
            let mip_levels = footer[36];
            let fourcc = &footer[108..112];
            let payload_len = decomp.len() - 120;

            let format = match fourcc {
                b"DXT1" => TextureFormat::Dxt1,
                b"DXT3" => TextureFormat::Dxt3,
                b"DXT5" => TextureFormat::Dxt5,
                b"RGBA" | b"ARGB" | b"BGRA" => TextureFormat::Rgba32,
                [0, 0, 0, 0] | [0x15, 0, 0, 0]
                    if width > 0
                        && height > 0
                        && (width as usize)
                            .checked_mul(height as usize)
                            .and_then(|px| px.checked_mul(4))
                            .is_some_and(|bytes| bytes <= payload_len) =>
                {
                    TextureFormat::Rgba32
                }
                other => TextureFormat::Unknown(other[0]),
            };

            let data = decomp[..payload_len].to_vec();
            pack.textures.push(TextureEntry {
                hash: desc.hash,
                width,
                height,
                mip_levels,
                format,
                data,
            });
        }
    }

    synthesize_vehicle_composite_textures(&mut pack);

    Ok(pack)
}

/// Composite vehicle skin (`CARSKIN` / `350Z_SKIN1`) from the `512x512` `*_DOORLINE` and `*_DOORLINE_MASK`
/// textures when present in a vehicle texture pack, and tint grayscale taillight lens maps.
fn synthesize_vehicle_composite_textures(pack: &mut TexturePack) {
    // Locate 350Z_DOORLINE (0x9F4B8D4C) and 350Z_DOORLINE_MASK (0x85B62D57)
    let doorline_opt = pack
        .textures
        .iter()
        .find(|t| {
            t.hash == 0x9F4B_8D4C
                && t.format == TextureFormat::Rgba32
                && t.width == 512
                && t.height == 512
        })
        .cloned();
    let mask_opt = pack
        .textures
        .iter()
        .find(|t| {
            t.hash == 0x85B6_2D57
                && t.format == TextureFormat::Rgba32
                && t.width == 512
                && t.height == 512
        })
        .cloned();

    if let (Some(dl), Some(mk)) = (doorline_opt, mask_opt) {
        let px_bytes = 512usize * 512usize * 4;
        if dl.data.len() >= px_bytes && mk.data.len() >= px_bytes {
            // Iconic NFSU2 Nissan 350Z Titanium Silver metallic base coat
            let (paint_r, paint_g, paint_b) = (168.0_f32, 176.0_f32, 188.0_f32);
            let mut bgra = vec![0u8; px_bytes];

            for ((dl_px, mk_px), out_px) in dl.data[..px_bytes]
                .chunks_exact(4)
                .zip(mk.data[..px_bytes].chunks_exact(4))
                .zip(bgra.chunks_exact_mut(4))
            {
                let b = dl_px[0] as f32;
                let g = dl_px[1] as f32;
                let r = dl_px[2] as f32;
                let a = dl_px[3] as f32;
                let m = (mk_px[0].min(128)) as f32;

                // Non-painted detail alpha (grille mesh, orange reflector, door panel, Z kickplate, reverse light)
                let detail_t = ((a - 53.0) / (255.0 - 53.0)).clamp(0.0, 1.0);
                // Panel seam darkening factor from DOORLINE_MASK (m = 30..34 on door/hood/trunk seams)
                let seam_shade = (1.0 - (m / 38.0)).clamp(0.12, 1.0);

                let pr = paint_r * seam_shade;
                let pg = paint_g * seam_shade;
                let pb = paint_b * seam_shade;

                let out_r = (pr * (1.0 - detail_t) + r * detail_t).clamp(0.0, 255.0) as u8;
                let out_g = (pg * (1.0 - detail_t) + g * detail_t).clamp(0.0, 255.0) as u8;
                let out_b = (pb * (1.0 - detail_t) + b * detail_t).clamp(0.0, 255.0) as u8;

                out_px[0] = out_b;
                out_px[1] = out_g;
                out_px[2] = out_r;
                out_px[3] = 255;
            }

            for skin_hash in [0x3C84_D757, 0xA675_4656, 0xD6D6_080A, 0xA736_6AE6] {
                if !pack.textures.iter().any(|t| t.hash == skin_hash) {
                    pack.textures.push(TextureEntry {
                        hash: skin_hash,
                        width: 512,
                        height: 512,
                        mip_levels: 1,
                        format: TextureFormat::Rgba32,
                        data: bgra.clone(),
                    });
                }
            }
        }
    }

    // Tint 350Z_KIT00_BRAKELIGHT_GLASS (0x9B4396C7) from grayscale lens mask (luma 20..120)
    // into a vibrant ruby-red ribbed taillight lens texture.
    if let Some(idx) = pack.textures.iter().position(|t| t.hash == 0x9B43_96C7) {
        if let Ok(rgba) = pack.textures[idx].decode_to_rgba8() {
            let mut bgra = vec![0u8; rgba.len()];
            for (src, dst) in rgba.chunks_exact(4).zip(bgra.chunks_exact_mut(4)) {
                let luma = src[0] as f32;
                let t = ((luma - 25.0) / (115.0 - 25.0)).clamp(0.0, 1.0);
                let r = (135.0 + t * 105.0) as u8;
                let g = (12.0 + t * 22.0) as u8;
                let b = (18.0 + t * 24.0) as u8;
                dst[0] = b;
                dst[1] = g;
                dst[2] = r;
                dst[3] = 255;
            }
            pack.textures[idx].format = TextureFormat::Rgba32;
            pack.textures[idx].mip_levels = 1;
            pack.textures[idx].data = bgra;
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct RawTextureDesc {
    hash: u32,
    offset: usize,
    comp_size: usize,
    _uncomp_size: usize,
}

fn parse_texture_info(
    payload: &[u8],
    pack: &mut TexturePack,
    descs: &mut Vec<RawTextureDesc>,
) -> Result<(), TextureError> {
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
                let offset = u32::from_le_bytes([
                    entry_data[4],
                    entry_data[5],
                    entry_data[6],
                    entry_data[7],
                ]) as usize;
                let comp_size = u32::from_le_bytes([
                    entry_data[8],
                    entry_data[9],
                    entry_data[10],
                    entry_data[11],
                ]) as usize;
                let _uncomp_size = u32::from_le_bytes([
                    entry_data[12],
                    entry_data[13],
                    entry_data[14],
                    entry_data[15],
                ]) as usize;

                descs.push(RawTextureDesc {
                    hash,
                    offset,
                    comp_size,
                    _uncomp_size,
                });
            }
        }
    }
    Ok(())
}

impl TextureEntry {
    /// Decode the texture's compressed pixel buffer into raw 32-bit RGBA (RGBA8) pixels.
    ///
    /// Decodes the primary base mip level (level 0).
    pub fn decode_to_rgba8(&self) -> Result<Vec<u8>, TextureError> {
        let w = self.width as usize;
        let h = self.height as usize;
        if w == 0 || h == 0 {
            return Err(TextureError::InvalidDimensions {
                width: w,
                height: h,
            });
        }

        let total_bytes =
            w.checked_mul(h)
                .and_then(|v| v.checked_mul(4))
                .ok_or(TextureError::Chunk(
                    crate::error::ChunkError::IntegerOverflow {
                        context: "rgba8 buffer size",
                    },
                ))?;
        let mut out = vec![0u8; total_bytes];

        match self.format {
            TextureFormat::Dxt1 => {
                decode_dxt1(&self.data, w, h, &mut out)?;
            }
            TextureFormat::Dxt3 => {
                decode_dxt3(&self.data, w, h, &mut out)?;
            }
            TextureFormat::Dxt5 => {
                decode_dxt5(&self.data, w, h, &mut out)?;
            }
            TextureFormat::Rgba32 => {
                if self.data.len() < total_bytes {
                    return Err(TextureError::UnexpectedEof {
                        offset: 0,
                        expected: total_bytes,
                        available: self.data.len(),
                    });
                }
                for (src, dst) in self.data.chunks_exact(4).zip(out.chunks_exact_mut(4)) {
                    // Convert BGRA to RGBA
                    dst[0] = src[2];
                    dst[1] = src[1];
                    dst[2] = src[0];
                    dst[3] = src[3];
                }
            }
            TextureFormat::Unknown(_) => {
                return Err(TextureError::UnsupportedFormat(self.format));
            }
        }

        Ok(out)
    }
}

#[inline]
fn unpack_rgb565(c: u16) -> [u8; 3] {
    let r = ((c >> 11) & 0x1F) as u32;
    let g = ((c >> 5) & 0x3F) as u32;
    let b = (c & 0x1F) as u32;
    [
        ((r * 255 + 15) / 31) as u8,
        ((g * 255 + 31) / 63) as u8,
        ((b * 255 + 15) / 31) as u8,
    ]
}

/// Decode a DXT1 (BC1) block-compressed buffer into raw RGBA8 pixels.
pub fn decode_dxt1(
    data: &[u8],
    width: usize,
    height: usize,
    out: &mut [u8],
) -> Result<(), TextureError> {
    if width == 0 || height == 0 {
        return Err(TextureError::InvalidDimensions { width, height });
    }
    let blocks_x = width.div_ceil(4);
    let blocks_y = height.div_ceil(4);
    let required_bytes = blocks_x
        .checked_mul(blocks_y)
        .and_then(|v| v.checked_mul(8))
        .ok_or(TextureError::Chunk(
            crate::error::ChunkError::IntegerOverflow {
                context: "dxt1 blocks",
            },
        ))?;

    if data.len() < required_bytes {
        return Err(TextureError::UnexpectedEof {
            offset: 0,
            expected: required_bytes,
            available: data.len(),
        });
    }

    let mut in_offset = 0;
    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            let block = &data[in_offset..in_offset + 8];
            in_offset += 8;

            let c0 = u16::from_le_bytes([block[0], block[1]]);
            let c1 = u16::from_le_bytes([block[2], block[3]]);
            let lookup = u32::from_le_bytes([block[4], block[5], block[6], block[7]]);

            let [r0, g0, b0] = unpack_rgb565(c0);
            let [r1, g1, b1] = unpack_rgb565(c1);

            let mut colors = [[0u8; 4]; 4];
            colors[0] = [r0, g0, b0, 255];
            colors[1] = [r1, g1, b1, 255];
            if c0 > c1 {
                colors[2] = [
                    ((2 * r0 as u32 + r1 as u32) / 3) as u8,
                    ((2 * g0 as u32 + g1 as u32) / 3) as u8,
                    ((2 * b0 as u32 + b1 as u32) / 3) as u8,
                    255,
                ];
                colors[3] = [
                    ((r0 as u32 + 2 * r1 as u32) / 3) as u8,
                    ((g0 as u32 + 2 * g1 as u32) / 3) as u8,
                    ((b0 as u32 + 2 * b1 as u32) / 3) as u8,
                    255,
                ];
            } else {
                colors[2] = [
                    ((r0 as u32 + r1 as u32) / 2) as u8,
                    ((g0 as u32 + g1 as u32) / 2) as u8,
                    ((b0 as u32 + b1 as u32) / 2) as u8,
                    255,
                ];
                colors[3] = [0, 0, 0, 0];
            }

            for py in 0..4 {
                let y = by * 4 + py;
                if y >= height {
                    continue;
                }
                for px in 0..4 {
                    let x = bx * 4 + px;
                    if x >= width {
                        continue;
                    }
                    let shift = 2 * (py * 4 + px);
                    let code = ((lookup >> shift) & 0x03) as usize;
                    let pixel_idx = (y * width + x) * 4;
                    out[pixel_idx..pixel_idx + 4].copy_from_slice(&colors[code]);
                }
            }
        }
    }

    Ok(())
}

/// Decode a DXT3 (BC2) block-compressed buffer into raw RGBA8 pixels.
pub fn decode_dxt3(
    data: &[u8],
    width: usize,
    height: usize,
    out: &mut [u8],
) -> Result<(), TextureError> {
    if width == 0 || height == 0 {
        return Err(TextureError::InvalidDimensions { width, height });
    }
    let blocks_x = width.div_ceil(4);
    let blocks_y = height.div_ceil(4);
    let required_bytes = blocks_x
        .checked_mul(blocks_y)
        .and_then(|v| v.checked_mul(16))
        .ok_or(TextureError::Chunk(
            crate::error::ChunkError::IntegerOverflow {
                context: "dxt3 blocks",
            },
        ))?;

    if data.len() < required_bytes {
        return Err(TextureError::UnexpectedEof {
            offset: 0,
            expected: required_bytes,
            available: data.len(),
        });
    }

    let mut in_offset = 0;
    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            let block = &data[in_offset..in_offset + 16];
            in_offset += 16;

            let alpha_bytes = &block[0..8];
            let c0 = u16::from_le_bytes([block[8], block[9]]);
            let c1 = u16::from_le_bytes([block[10], block[11]]);
            let lookup = u32::from_le_bytes([block[12], block[13], block[14], block[15]]);

            let [r0, g0, b0] = unpack_rgb565(c0);
            let [r1, g1, b1] = unpack_rgb565(c1);

            let mut colors = [[0u8; 3]; 4];
            colors[0] = [r0, g0, b0];
            colors[1] = [r1, g1, b1];
            colors[2] = [
                ((2 * r0 as u32 + r1 as u32) / 3) as u8,
                ((2 * g0 as u32 + g1 as u32) / 3) as u8,
                ((2 * b0 as u32 + b1 as u32) / 3) as u8,
            ];
            colors[3] = [
                ((r0 as u32 + 2 * r1 as u32) / 3) as u8,
                ((g0 as u32 + 2 * g1 as u32) / 3) as u8,
                ((b0 as u32 + 2 * b1 as u32) / 3) as u8,
            ];

            for py in 0..4 {
                let y = by * 4 + py;
                if y >= height {
                    continue;
                }
                for px in 0..4 {
                    let x = bx * 4 + px;
                    if x >= width {
                        continue;
                    }
                    let p_idx = py * 4 + px;
                    let a_byte = alpha_bytes[p_idx / 2];
                    let a = if p_idx % 2 == 0 {
                        (a_byte & 0x0F) * 17
                    } else {
                        ((a_byte >> 4) & 0x0F) * 17
                    };

                    let shift = 2 * p_idx;
                    let code = ((lookup >> shift) & 0x03) as usize;
                    let pixel_idx = (y * width + x) * 4;
                    let [r, g, b] = colors[code];
                    out[pixel_idx] = r;
                    out[pixel_idx + 1] = g;
                    out[pixel_idx + 2] = b;
                    out[pixel_idx + 3] = a;
                }
            }
        }
    }

    Ok(())
}

/// Decode a DXT5 (BC3) block-compressed buffer into raw RGBA8 pixels.
pub fn decode_dxt5(
    data: &[u8],
    width: usize,
    height: usize,
    out: &mut [u8],
) -> Result<(), TextureError> {
    if width == 0 || height == 0 {
        return Err(TextureError::InvalidDimensions { width, height });
    }
    let blocks_x = width.div_ceil(4);
    let blocks_y = height.div_ceil(4);
    let required_bytes = blocks_x
        .checked_mul(blocks_y)
        .and_then(|v| v.checked_mul(16))
        .ok_or(TextureError::Chunk(
            crate::error::ChunkError::IntegerOverflow {
                context: "dxt5 blocks",
            },
        ))?;

    if data.len() < required_bytes {
        return Err(TextureError::UnexpectedEof {
            offset: 0,
            expected: required_bytes,
            available: data.len(),
        });
    }

    let mut in_offset = 0;
    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            let block = &data[in_offset..in_offset + 16];
            in_offset += 16;

            let a0 = block[0];
            let a1 = block[1];
            let mut alphas = [0u8; 8];
            alphas[0] = a0;
            alphas[1] = a1;
            if a0 > a1 {
                for i in 1..=6 {
                    let iu = i as u32;
                    alphas[i + 1] = (((7 - iu) * a0 as u32 + iu * a1 as u32) / 7) as u8;
                }
            } else {
                for i in 1..=4 {
                    let iu = i as u32;
                    alphas[i + 1] = (((5 - iu) * a0 as u32 + iu * a1 as u32) / 5) as u8;
                }
                alphas[6] = 0;
                alphas[7] = 255;
            }

            let alpha_bits = u64::from_le_bytes([
                block[2], block[3], block[4], block[5], block[6], block[7], 0, 0,
            ]);

            let c0 = u16::from_le_bytes([block[8], block[9]]);
            let c1 = u16::from_le_bytes([block[10], block[11]]);
            let lookup = u32::from_le_bytes([block[12], block[13], block[14], block[15]]);

            let [r0, g0, b0] = unpack_rgb565(c0);
            let [r1, g1, b1] = unpack_rgb565(c1);

            let mut colors = [[0u8; 3]; 4];
            colors[0] = [r0, g0, b0];
            colors[1] = [r1, g1, b1];
            colors[2] = [
                ((2 * r0 as u32 + r1 as u32) / 3) as u8,
                ((2 * g0 as u32 + g1 as u32) / 3) as u8,
                ((2 * b0 as u32 + b1 as u32) / 3) as u8,
            ];
            colors[3] = [
                ((r0 as u32 + 2 * r1 as u32) / 3) as u8,
                ((g0 as u32 + 2 * g1 as u32) / 3) as u8,
                ((b0 as u32 + 2 * b1 as u32) / 3) as u8,
            ];

            for py in 0..4 {
                let y = by * 4 + py;
                if y >= height {
                    continue;
                }
                for px in 0..4 {
                    let x = bx * 4 + px;
                    if x >= width {
                        continue;
                    }
                    let p_idx = py * 4 + px;
                    let a_code = ((alpha_bits >> (3 * p_idx)) & 0x07) as usize;
                    let a = alphas[a_code];

                    let shift = 2 * p_idx;
                    let code = ((lookup >> shift) & 0x03) as usize;
                    let pixel_idx = (y * width + x) * 4;
                    let [r, g, b] = colors[code];
                    out[pixel_idx] = r;
                    out[pixel_idx + 1] = g;
                    out[pixel_idx + 2] = b;
                    out[pixel_idx + 3] = a;
                }
            }
        }
    }

    Ok(())
}
