# EA Texture Pack & JDLZ Compression Format (`TEXTURES.BIN`) Specification

- **Status:** Confirmed by reverse engineering `SPEED2.EXE` (`0x00494e60`, `0x00494340`, `0x00494a10`, `0x005ce120`, `0x005745c0`, `0x00573650`)
- **Applies to:** Vehicle texture archives (`game-assets/CARS/*/TEXTURES.BIN`, `CARS/*/VINYLS.BIN`, `GLOBAL/*.BIN`)
- **Byte Order:** Little-Endian (`<`)
- **Version:** Need for Speed Underground 2 (PC v1.2 reference)

---

## 1. Texture Pack Architecture

Textures are packed into self-contained archives under chunk ID `0xB3300000`.

### Chunk Hierarchy:

```text
0xB3300000: TEXTURE_PACK_CONTAINER
├── 0xB3310000: TEXTURE_INFO_CONTAINER
│   ├── 0x33310001: TEXTURE_PACK_HEADER (124 bytes: Package name, hashes)
│   ├── 0x33310002: TEXTURE_HASH_TABLE (8 bytes per entry: [hash, 0])
│   └── 0x33310003: TEXTURE_METADATA_TABLE (24 bytes per entry)
└── 0xB3320000: TEXTURE_DATA_CONTAINER
    ├── 0x33320001: TEXTURE_DATA_HEADER (24 bytes)
    └── 0x33320002: TEXTURE_DATA_STREAM (128-byte aligned raw DXT or JDLZ blocks)
```

---

## 2. Detailed Chunk Specifications

### 2.1. Texture Pack Header (`0x33310001`)

Size is 124 bytes (`0x7C`):

| Offset | Type | Field Name | Description / Sample Value |
| :--- | :--- | :--- | :--- |
| `0x00` | `u32` | `version` | Package format version (`0x00000005`) |
| `0x04..0x0F` | `[u8; 12]` | `tag` | ASCII Tag `"TRACEXTURES\0"` |
| `0x10..0x1F` | `[u8; 16]` | `reserved_0` | Zeroed |
| `0x20..0x5F` | `[u8; 64]` | `package_name` | Source path (e.g. `"Global\\Pipeline\\TrackTemplateTextures_350Z.tpk\0"`) |
| `0x60` | `u32` | `package_hash` | Hash of the texture pack identifier |
| `0x64..0x7B` | `[u8; 24]` | `reserved_1` | Zeroed padding |

### 2.2. Texture Hash Table (`0x33310002`)

Array of 8-byte entries (`size / 8 = texture_count`):
- `0x00..0x03`: `texture_hash: u32` (EA binHash32 of texture name).
- `0x04..0x07`: `u32` (Zeroed padding / runtime pointer slot).

In the sample `350Z/TEXTURES.BIN`, `size = 584` bytes → **73 textures**.

### 2.3. Texture Metadata Descriptors (`0x33310003`)

Array of 24-byte (`0x18`) records matching the hash table count (`size / 24 = texture_count`):

| Offset | Type | Field Name | Description |
| :--- | :--- | :--- | :--- |
| `0x00` | `u32` (LE) | `texture_hash` | Hash matching the entry in `0x33310002` |
| `0x04` | `u32` (LE) | `data_offset` | Byte offset into the texture payload data stream |
| `0x08` | `u32` (LE) | `data_size` | Length in bytes of the compressed or raw texture payload |
| `0x0C` | `u32` (LE) | `flags` | Texture format and compression flags (bit `0x02` = JDLZ compressed) |
| `0x10` | `u16` (LE) | `width` | Texture width in pixels (e.g., `128`, `256`, `512`) |
| `0x12` | `u16` (LE) | `height` | Texture height in pixels (e.g., `128`, `256`, `512`) |
| `0x14` | `u8` | `mip_levels` | Number of mipmap levels present |
| `0x15` | `u8` | `pixel_format` | Internal format enum (`0x29` = DXT3, `0x27` = DXT1, `0x28` = ARGB) |
| `0x16..0x17` | `[u8; 2]` | `padding` | Alignment padding |

---

## 3. JDLZ Proprietary Decompression Algorithm

Textures whose data payloads start with the magic sequence `b"JDLZ"` are compressed using EA's proprietary variant of LZSS.
The algorithm was decompiled directly from `SPEED2.EXE` (`func_0x00573650`).

### 3.1. JDLZ Header Layout

The JDLZ stream begins with a 16-byte header:

| Offset | Type | Field Name | Description |
| :--- | :--- | :--- | :--- |
| `0x00..0x03` | `[u8; 4]` | `magic` | Exact magic bytes `b"JDLZ"` (`0x5A4C444A`) |
| `0x04` | `u8` | `version` | Must equal `0x02` |
| `0x05..0x07` | `[u8; 3]` | `reserved` | Header padding |
| `0x08` | `u32` (LE) | `uncompressed_size` | Expected size of decoded buffer in bytes |
| `0x0C` | `u32` (LE) | `compressed_size` | Total length of compressed block including 16-byte header |

### 3.2. Decompression Logic (Pseudocode)

```python
def decompress_jdlz(data: bytes) -> bytes:
    magic = data[0:4]
    version = data[4]
    assert magic == b"JDLZ" and version == 2

    uncomp_len = int.from_bytes(data[8:12], "little")
    comp_len = int.from_bytes(data[12:16], "little")

    out = bytearray()
    in_idx = 16
    flags1 = data[in_idx] | 0x100
    in_idx += 1
    flags2 = data[in_idx] | 0x100
    in_idx += 1

    bytes_left = comp_len - 18

    while bytes_left > 0 and len(out) < uncomp_len:
        if flags1 & 1:
            # Back-reference match
            if flags2 & 1:
                b1 = data[in_idx]
                b2 = data[in_idx + 1]
                in_idx += 2
                bytes_left -= 2
                length = (((b1 & 0xF0) << 4) | b2) + 3
                offset = (b1 & 0x0F) + 1
            else:
                b1 = data[in_idx]
                b2 = data[in_idx + 1]
                in_idx += 2
                bytes_left -= 2
                offset = (((b1 & 0xE0) << 3) | b2) + 17
                length = (b1 & 0x1F) + 3

            flags2 >>= 1
            for _ in range(length):
                out.append(out[-offset])
        else:
            # Literal byte copy
            out.append(data[in_idx])
            in_idx += 1
            bytes_left -= 1

        flags1 >>= 1
        if flags1 == 1:
            if bytes_left > 0:
                flags1 = data[in_idx] | 0x100
                in_idx += 1
                bytes_left -= 1
        if flags2 == 1:
            if bytes_left > 0:
                flags2 = data[in_idx] | 0x100
                in_idx += 1
                bytes_left -= 1

    return bytes(out)
```

---

## 4. Texture Formats (DXT1 / DXT3 / DXT5)

Once uncompressed, texture data streams consist of raw DirectX Texture blocks:

1. **DXT1 (`BC1`)**:
   - 8 bytes per 4×4 pixel block.
   - 2 RGB565 endpoints (4 bytes) + 4×4 2-bit index table (4 bytes).
   - Used for opaque body paint and decals without smooth alpha.
2. **DXT3 (`BC2`)**:
   - 16 bytes per 4×4 pixel block.
   - 4-bit explicit alpha per pixel (8 bytes) + DXT1 color block (8 bytes).
   - Used for windows, badges, brake lights, and transparent decals.

---

## 5. Reviewed Rust Parser Design (M1-05 Gate)

```rust
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureFormat {
    Dxt1,
    Dxt3,
    Dxt5,
    Rgba32,
}

#[derive(Debug, Clone)]
pub struct TextureMetadata {
    pub hash: u32,
    pub width: u16,
    pub height: u16,
    pub mip_levels: u8,
    pub format: TextureFormat,
    pub data_offset: usize,
    pub data_size: usize,
    pub is_compressed: bool,
}

#[derive(Debug, Error)]
pub enum TextureError {
    #[error("Corrupt JDLZ header: expected magic 'JDLZ', found {0:?}")]
    InvalidJdlzMagic([u8; 4]),

    #[error("Unsupported JDLZ version: expected 2, found {0}")]
    UnsupportedJdlzVersion(u8),

    #[error("JDLZ buffer overflow: decompressed data exceeded expected {0} bytes")]
    DecompressionOverflow(usize),

    #[error("JDLZ back-reference out of bounds: offset {offset} with output size {out_len}")]
    InvalidBackReference { offset: usize, out_len: usize },

    #[error("Unexpected EOF reading texture data at offset {0:#x}")]
    UnexpectedEof(usize),
}
```
