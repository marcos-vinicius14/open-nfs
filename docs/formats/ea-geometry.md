# EA Vehicle Geometry Format (`GEOMETRY.BIN`) Specification

- **Status:** Confirmed by reverse engineering `SPEED2.EXE` (`0x00493b70`, `0x005cc1d0`, `0x005c57f0`)
- **Applies to:** Vehicle geometry files (`game-assets/CARS/*/GEOMETRY.BIN`, `CARS/WHEELS/GEOMETRY_*.BIN`, `CARS/BRAKES/GEOMETRY.BIN`)
- **Byte Order:** Little-Endian (`<`)
- **Version:** Need for Speed Underground 2 (PC v1.2 reference)

---

## 1. General EA Chunk Container Architecture

All geometry and texture archives in Need for Speed Underground 2 use the proprietary EA Chunk Container system.

### 1.1. Chunk Header Layout

Every chunk begins with an 8-byte header:

| Offset | Type | Field Name | Description |
| :--- | :--- | :--- | :--- |
| `0x00` | `u32` (LE) | `chunk_id` | 32-bit chunk type identifier |
| `0x04` | `u32` (LE) | `chunk_size` | Length of chunk payload in bytes (excludes the 8-byte header) |

### 1.2. Chunk Semantics & Invariants

1. **Container vs. Leaf Chunks**:
   - Chunks where `(chunk_id & 0x80000000) != 0` are **container chunks**. Their payload contains nested subchunks.
   - Chunks where `(chunk_id & 0x80000000) == 0` are **leaf/data chunks**. Their payload contains structured binary records or raw vertex/index data.
2. **Sequential Traversal**:
   - The next sibling chunk begins at `current_chunk_offset + 8 + chunk_size`.
3. **Padding and Sentinels**:
   - Chunks are frequently aligned to 4, 16, or 128-byte boundaries.
   - Alignment padding is filled with the sentinel byte `0x11` (or chunk ID `0x00000000` with the padding size).
   - Parsers must discard padding bytes between chunks rather than failing.

---

## 2. Vehicle Geometry Chunk Hierarchy

```text
0x80134000: SOLID_CONTAINER (Root Car Geometry Container)
├── 0x80134001: SOLID_HEADER_CONTAINER
│   ├── 0x00134002: Car File Metadata (144 bytes)
│   ├── 0x00134003: Car File Properties (4,856 bytes)
│   ├── 0x00134004: Car Part Index / Hash Table (14,568 bytes)
│   └── 0x80134008: Header Terminator Container
└── [Repeated Car Parts]
    └── 0x80134010: SOLID_PART_CONTAINER (One per visual component / LOD)
        ├── 0x00134011: SOLID_PART_HEADER (192 bytes: Part name, bounds, flags)
        ├── 0x00134012: Part Transform Matrix / Pivot
        ├── 0x00134013: Part Mounting Points
        ├── 0x0013401A: Animation / Kinematics Descriptor
        └── 0x80134100: SOLID_MESH_CONTAINER
            ├── 0x00134900: SOLID_MESH_DESCRIPTOR (68 bytes: Counts & limits)
            ├── 0x00134B01: SOLID_VERTEX_STREAM (128-byte aligned, 36B/vertex)
            ├── 0x00134B02: SOLID_INDEX_STREAM (60B/submesh draw call slice)
            └── 0x00134B03: SOLID_TRIANGLES (16-bit triangle indices)
```

---

## 3. Detailed Chunk Specifications

### 3.1. Car Part Header (`0x00134011`)

Each car part container (`0x80134010`) contains exactly one part header chunk (`0x00134011`). The payload is exactly 192 bytes (`0xC0`):

| Offset | Type | Field Name | Description / Validated Value |
| :--- | :--- | :--- | :--- |
| `0x00..0x0B` | `[u8; 12]` | `reserved_0` | Padding / zeroed |
| `0x0C` | `u16` (LE) | `part_flags` | Part visibility and render flags |
| `0x0E` | `u16` (LE) | `lod_level` | Visual Level-Of-Detail (`0` = High/A, `1` = Med/B, `2` = Low/C, `3` = Tiny/D) |
| `0x10` | `u32` (LE) | `part_hash` | EA binHash32 of the part name string |
| `0x14` | `u32` (LE) | `triangle_count` | Total triangles in this part |
| `0x18` | `u16` (LE) | `vertex_count` | Total vertices in this part |
| `0x1A` | `u8` | `submesh_count` | Number of draw calls / material slices |
| `0x1B` | `u8` | `bone_count` | Number of skinning bones (or 0 for rigid meshes) |
| `0x20..0x2C` | `[f32; 3]` | `bbox_min` | Bounding box minimum `[x, y, z]` in meters |
| `0x30..0x3C` | `[f32; 3]` | `bbox_max` | Bounding box maximum `[x, y, z]` in meters |
| `0x40..0x5F` | `[u8; 32]` | `part_name` | Null-terminated ASCII string (e.g. `"350Z_BASE_A\0"`, `"KIT00_FRONT_BUMPER_A\0"`) |
| `0x60..0xBF` | `[u8; 96]` | `unknown_padding` | Material bind indices and animation parameters |

### 3.2. Mesh Descriptor (`0x00134900`)

The mesh descriptor specifies the global buffer counts for the part. Payload size is 68 bytes (`0x44`):

| Offset | Type | Field Name | Evidence / Value in Sample (`350Z_BASE_A`) |
| :--- | :--- | :--- | :--- |
| `0x00..0x07` | `[u8; 8]` | `reserved_0` | Zeroed |
| `0x08` | `u32` (LE) | `alignment_flags` | Alignment hint (`0x00000010` = 16-byte index align) |
| `0x0C` | `u32` (LE) | `material_flags` | Default shader flags (`0x0000C180` in sample) |
| `0x10` | `u32` (LE) | `num_submeshes` | Count of submesh entries in chunk `0x00134B02` (`11`) |
| `0x14..0x23` | `[u8; 16]` | `reserved_1` | Zeroed |
| `0x24` | `u32` (LE) | `num_triangles` | Total triangle count (`500` in sample) |
| `0x28..0x33` | `[u8; 12]` | `reserved_2` | Zeroed |
| `0x34` | `u32` (LE) | `num_vertices` | Total vertex count (`657` in sample) |
| `0x38..0x43` | `[u8; 12]` | `reserved_3` | Zeroed |

### 3.3. Vertex Stream (`0x00134B01`)

- **Alignment**: The vertex stream payload in the file is aligned to a **128-byte boundary** (SIMD / cache-line alignment in `SPEED2.EXE` at `0x005cc2de`):
  `vertex_payload_offset = (chunk_header_start + 0x87) & !0x7F`
- **Vertex Stride**: Exactly **36 bytes** per vertex (`#[repr(C)]`).
- **Total Validated Payload Size**: `num_vertices * 36` bytes.

#### Vertex 36-Byte Structure:

| Offset | Type | Field Name | Description | Verified Range in Reference |
| :--- | :--- | :--- | :--- | :--- |
| `0x00` | `f32` (LE) | `pos_x` | Local X position in meters | `[-2.10, +2.10]` |
| `0x04` | `f32` (LE) | `pos_y` | Local Y position in meters | `[-0.90, +0.90]` |
| `0x08` | `f32` (LE) | `pos_z` | Local Z position in meters (up) | `[+0.05, +1.40]` |
| `0x0C` | `f32` (LE) | `normal_x` | Surface normal vector X | Unit magnitude (`sqrt(nx²+ny²+nz²) = 1.0000`) |
| `0x10` | `f32` (LE) | `normal_y` | Surface normal vector Y | Unit magnitude |
| `0x14` | `f32` (LE) | `normal_z` | Surface normal vector Z | Unit magnitude |
| `0x18` | `u32` (LE) | `color` | Packed ARGB diffuse vertex color | Default `0xFF000000` |
| `0x1C` | `f32` (LE) | `tex_u` | U texture coordinate | Normalized `[0.0, 1.0]` |
| `0x20` | `f32` (LE) | `tex_v` | V texture coordinate | Normalized `[0.0, 1.0]` |

### 3.4. Sub-mesh / Draw Slice Descriptors (`0x00134B02`)

Chunk `0x00134B02` contains a contiguous array of submesh descriptors. Each entry is **60 bytes** (`0x3C`):

| Offset | Type | Field Name | Description |
| :--- | :--- | :--- | :--- |
| `0x00..0x0B` | `[f32; 3]` | `submesh_min` | Bounding box minimum `[x, y, z]` for this draw call slice |
| `0x0C` | `u32` (LE) | `index_count` | Number of 16-bit indices for this draw call (e.g. `18`, `24`, `96`, `942`) |
| `0x10..0x1B` | `[f32; 3]` | `submesh_max` | Bounding box maximum `[x, y, z]` for this draw call slice |
| `0x1C` | `u32` (LE) | `material_id` | Material slot index binding into the texture pack |
| `0x20..0x33` | `[u8; 20]` | `unknown_state` | Render state flags, alpha blend flags |
| `0x34` | `u32` (LE) | `index_offset` | Starting index offset in the index buffer (in units of `u16` indices) |
| `0x38` | `u32` (LE) | `shader_flags` | Shader feature mask (e.g. `0x00004180`, `0x00004080`, `0x0000C180`) |

### 3.5. Triangle Indices (`0x00134B03`)

- **Alignment**: Aligned to a 16-byte boundary:
  `index_payload_offset = (chunk_header_start + 0x17) & !0xF`
- **Encoding**: 16-bit unsigned integers (`u16`, little-endian).
- **Topology**: Triangle list (every 3 consecutive indices form one triangle; `triangle_count = index_count / 3`).
- **Sentinel Padding**: The alignment gap between chunk header and payload is padded with `0x1111` sentinels.

---

## 4. Reviewed Rust Parser Design (M1-05 Gate)

Following the standards in `AGENTS.md` (zero panics, bounds-checked, explicit endianness), the Rust parser interface in `crates/asset` will model the domain structures as follows:

```rust
use bytemuck::{Pod, Zeroable};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkId(pub u32);

impl ChunkId {
    pub const SOLID_CONTAINER: Self = Self(0x8013_4000);
    pub const SOLID_HEADER_CONTAINER: Self = Self(0x8013_4001);
    pub const SOLID_PART_CONTAINER: Self = Self(0x8013_4010);
    pub const SOLID_PART_HEADER: Self = Self(0x0013_4011);
    pub const SOLID_MESH_CONTAINER: Self = Self(0x8013_4100);
    pub const SOLID_MESH_DESCRIPTOR: Self = Self(0x0013_4900);
    pub const SOLID_VERTEX_STREAM: Self = Self(0x0013_4B01);
    pub const SOLID_INDEX_STREAM: Self = Self(0x0013_4B02);
    pub const SOLID_TRIANGLES: Self = Self(0x0013_4B03);

    #[inline]
    pub const fn is_container(self) -> bool {
        (self.0 & 0x8000_0000) != 0
    }
}

/// 36-byte standard DirectX vertex layout used in vehicle geometries
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct Vertex36 {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: u32,
    pub uv: [f32; 2],
}

/// 60-byte submesh draw-call record
#[derive(Debug, Clone, PartialEq)]
pub struct SubMeshSlice {
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
    pub index_count: u32,
    pub index_offset: u32,
    pub material_id: u32,
    pub shader_flags: u32,
}

#[derive(Debug, Clone)]
pub struct ParsedCarPart {
    pub name: String,
    pub lod_level: u16,
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
    pub vertices: Vec<Vertex36>,
    pub indices: Vec<u16>,
    pub submeshes: Vec<SubMeshSlice>,
}

#[derive(Debug, Error)]
pub enum GeometryParseError {
    #[error("Buffer truncated: expected {expected} bytes at offset {offset:#x}, available {available}")]
    UnexpectedEof { offset: usize, expected: usize, available: usize },

    #[error("Invalid container bounds: chunk size {size} exceeds buffer length {remaining}")]
    InvalidChunkBounds { size: usize, remaining: usize },

    #[error("Corrupt vertex stride: payload {payload_bytes} bytes is not a multiple of 36")]
    InvalidVertexStride { payload_bytes: usize },

    #[error("Index out of bounds: index {index} >= vertex count {vertex_count}")]
    IndexOutOfBounds { index: u16, vertex_count: u32 },

    #[error("Missing mandatory chunk {0:?} in car part container")]
    MissingChunk(ChunkId),
}
```
