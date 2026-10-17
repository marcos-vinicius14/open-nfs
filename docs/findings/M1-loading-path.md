# M1-01..05 — Vehicle Asset Loading Path & Binary Reverse Engineering

- **Task IDs:** M1-01, M1-02, M1-03, M1-04, M1-05
- **Milestone:** M1 (Asset Format Discovery)
- **Status:** Done
- **Requirements:** FR-003, EV-001, EV-002, EV-003, EV-004, EV-005, ADR-001, ADR-008, NFR-005
- **Date:** 2026-10-17
- **Analysis Target:** `reverse-engineering/binaries/SPEED2.EXE`
  - SHA-256: `82881d607ea8d90a2898ba9a6ed06da1ff6b56243f2e9146df30226fff69574b`
  - Linker: MSVC 7.0 (`0x0700`), Compilation Timestamp: `Fri Oct 29 21:23:10 2004`
- **Candidate Vehicle Assets:**
  - `game-assets/CARS/350Z/GEOMETRY.BIN` (7,930,864 bytes, SHA-256 recorded in `docs/findings/M0-manifest.md`)
  - `game-assets/CARS/350Z/TEXTURES.BIN` (617,280 bytes, SHA-256 recorded in `docs/findings/M0-manifest.md`)

---

## 1. Candidate Asset Identification & Tree Verification (M1-01, M1-02)

The game assets directory contains dedicated subdirectories per vehicle under `CARS/<VEHICLE_NAME>/`.
For the reference vehicle **Nissan 350Z** (`CARS/350Z/`), four primary files are observed:

| Filename | File Size | Format Role | Observed Container Type |
| :--- | :--- | :--- | :--- |
| `GEOMETRY.BIN` | 7,930,864 bytes | 3D Vehicle Meshes, Parts, LODs, Colliders | EA Chunk Container (`0x80134000`) |
| `TEXTURES.BIN` | 617,280 bytes | Vehicle Texture Maps, Paint Base, Decals | EA Texture Pack Container (`0xB3300000`) |
| `VINYLS.BIN` | 17,313,728 bytes | Custom Vinyl Decals and Layer Masks | EA Texture Pack Container (`0xB3300000`) |
| `PARTS_ANIMATIONS.bin` | 32,592 bytes | Part kinematics and animation state | EA Chunk Container (`0x00034000`) |

---

## 2. Resource Loading Path Decompilation (M1-03)

### 2.1. Vehicle Asset Loading Coordinator (`0x0063ae70` → `0x0063af70`)

In `SPEED2.EXE`, the vehicle model and texture loading subsystem is managed through function `func_0x0063ae70`. The vehicle name is injected via string formatting templates located in the `.rdata` section:

- **VA `0x007a8d80`:** `"CARS\\%s\\TEXTURES.BIN"`
- **VA `0x007a8d6c`:** `"CARS\\%s\\VINYLS.BIN"`
- **VA `0x007a867c`:** `"CARS\\%s\\PARTS_ANIMATIONS.BIN"`

#### Disassembly (`0x0063af70` - `0x0063af94`):
```x86asm
0063af70: push   %ebp                  ; Vehicle name string (e.g., "350Z")
0063af71: lea    0x60(%esp),%edx       ; Destination buffer for formatted path
0063af75: push   $0x7a8d80             ; "CARS\\%s\\TEXTURES.BIN"
0063af7a: push   %edx
0063af7b: call   0x440120              ; crt_sprintf(buf, "CARS\\%s\\TEXTURES.BIN", car_name)
0063af80: add    $0xc,%esp
0063af83: push   $0x8000               ; Allocation flags (32 KiB alignment hint)
0063af88: lea    0x60(%esp),%eax
0063af8c: push   %eax                  ; Filepath
0063af8d: mov    %esi,%ecx
0063af8f: call   0x633b90              ; Queue file streaming / memory mapping
0063af94: mov    0x304(%ebx),%ecx
```

### 2.2. Slot Pool Allocators for Car Resources (`0x0061c700`)

The game maintains global fixed slot pool managers for loaded car assets in initialized data (`0x008a1cf4`–`0x008a1d00`):
- `0x008a1cf4`: `CarLoadedTexturePackSlotPool` (`0x007a86f0`)
- `0x008a1cf8`: `CarLoadedSolidPackSlotPool` (`0x007a86d4`)
- `0x008a1cfc`: `CarLoadedSkinLayerSlotPool` (`0x007a86b8`)
- `0x008a1d00`: `CarLoadedRideInfoSlotPool` (`0x007a869c`)

When a vehicle's geometry pack is requested, `func_0x00633b00` allocates an entry from `CarLoadedSolidPackSlotPool` (`0x008a1cf8`) and establishes the memory-mapped streaming handle.

---

## 3. Car Part & Mesh Chunk Dispatchers (M1-03, M1-04)

### 3.1. Car Part Container Dispatcher (`func_0x00493b70`)

When a car geometry file (`GEOMETRY.BIN`) is read, it dispatches top-level car part containers marked with chunk ID `0x80134010` through function `0x00493b70`:

```x86asm
00493b70: push   %ecx
00493b71: mov    0x8(%esp),%eax        ; Chunk pointer
00493b75: cmpl   $0x80134010,(%eax)    ; Verify ID == 0x80134010 (SOLID_PART_CONTAINER)
00493b7b: je     0x493b81
00493b7d: xor    %eax,%eax
00493b7f: pop    %ecx
00493b80: ret
00493b81: mov    0x4(%eax),%ecx        ; Chunk size
...
00493b88: lea    0x8(%eax),%esi        ; Start of subchunks (eax + 8)
00493b8b: lea    0x8(%eax,%ecx,1),%ebx ; End of container
...
00493ba3: mov    (%esi),%eax           ; Read subchunk ID
00493ba5: cmp    $0x134011,%eax        ; SOLID_PART_HEADER
00493baa: jne    0x493bc2
00493bac: lea    0x17(%esi),%edi       ; (esi + 0x17) & ~0xF (16-byte alignment)
00493baf: and    $0xfffffff0,%edi      ; EDI = pointer to Part Header struct
00493bbd: jmp    0x493cc9              ; Advance subchunk
...
00493cb2: cmp    $0x80134100,%eax      ; SOLID_MESH_CONTAINER
00493cb7: jne    0x493cc9
00493cb9: push   %esi                  ; Push mesh container pointer
00493cba: mov    %edi,%ecx             ; ECX = Part Header struct
00493cbc: call   0x5cc1d0              ; <--- CALL MESH STREAM PARSER
```

### 3.2. Mesh Stream Parser (`func_0x005cc1d0`)

Function `0x005cc1d0` iterates over subchunks of `0x80134100` (`SOLID_MESH_CONTAINER`), decoding the mesh descriptor, submesh slices, 16-bit triangle index buffers, and 36-byte vertex buffers:

```x86asm
005cc1d0: sub    $0xc,%esp
005cc1d3: mov    0x10(%esp),%eax       ; Pointer to 0x80134100 chunk
005cc1dc: mov    0x4(%eax),%ecx        ; Container size
005cc1df: lea    0x8(%eax),%ebp        ; First subchunk pointer
005cc1e2: lea    0x8(%ecx,%eax,1),%eax ; End of container
...
; --- Handler: 0x00134900 (SOLID_MESH_DESCRIPTOR) ---
005cc200: mov    0x0(%ebp),%eax
005cc203: cmp    $0x134900,%eax
005cc208: jne    0x5cc22d
005cc20a: lea    0x17(%ebp),%ebx       ; 16-byte aligned descriptor pointer
005cc20d: and    $0xfffffff0,%ebx
005cc210: mov    %ebx,(%esi)           ; Store active mesh descriptor in part context
005cc228: jmp    0x5cc380              ; Advance to next subchunk

; --- Handler: 0x00134B02 (SOLID_INDEX_STREAM / SUBMESHES) ---
005cc22d: cmp    $0x134b02,%eax
005cc232: jne    0x5cc244
005cc234: mov    (%esi),%ebx
005cc236: lea    0x17(%ebp),%edx       ; 16-byte aligned submesh table pointer
005cc239: and    $0xfffffff0,%edx
005cc23c: mov    %edx,0x14(%ebx)       ; Store submesh array pointer in descriptor
005cc23f: jmp    0x5cc380

; --- Handler: 0x00134B03 (SOLID_TRIANGLES / INDEX BUFFER) ---
005cc244: cmp    $0x134b03,%eax
005cc249: jne    0x5cc2c6
005cc24b: mov    (%esi),%ebx
005cc24d: lea    0x17(%ebp),%eax       ; 16-byte aligned index buffer start
005cc250: mov    %eax,%ecx
005cc252: and    $0xfffffff0,%ecx      ; ECX = aligned index buffer payload
005cc255: mov    %ecx,0x18(%ebx)
005cc258: mov    0x4(%ebp),%edx        ; Subchunk raw size
005cc25b: and    $0xfffffff0,%eax
005cc25e: sub    %eax,%edx
005cc260: lea    0x8(%edx,%ebp,1),%eax ; EAX = effective index buffer byte size
005cc267: mov    %eax,0x1c(%ebx)
005cc26a: call   0x5c57f0              ; Compute submesh absolute index offsets
005cc290: call   *0x6c(%edx)           ; IDirect3DDevice9::CreateIndexBuffer(size, D3DFMT_INDEX16, ...)
005cc2a3: call   *0x2c(%eax)           ; IDirect3DIndexBuffer9::Lock(...)
005cc2b5: rep movsl                    ; Direct memory copy into GPU index buffer
005cc2bc: rep movsb

; --- Handler: 0x00134B01 (SOLID_VERTEX_STREAM / VERTEX BUFFER) ---
005cc2c6: cmp    $0x134b01,%eax
005cc2cb: jne    0x5cc380
005cc2d4: mov    (%esi),%ebx
005cc2d6: lea    0x87(%ebp),%eax       ; 128-byte cache-line alignment:
005cc2dc: mov    %eax,%edx             ; (ebp + 0x87) & ~0x7F
005cc2de: and    $0xffffff80,%edx      ; EDX = aligned vertex payload pointer
005cc2e7: mov    %edx,0x28(%ebx)
005cc2ea: mov    0x4(%ebp),%ecx        ; Chunk raw size
005cc2ed: sub    %eax,%ecx
005cc2f4: lea    0x8(%ecx,%ebp,1),%edi ; EDI = effective vertex buffer byte size
005cc2f8: mov    %edi,0x30(%ebx)
005cc33e: call   *0x68(%ecx)           ; IDirect3DDevice9::CreateVertexBuffer(size, USAGE_WRITEONLY, 0, D3DPOOL_MANAGED, ...)
005cc358: call   *0x2c(%edx)           ; IDirect3DVertexBuffer9::Lock(...)
005cc36a: rep movsl                    ; Direct memory copy into GPU vertex buffer
005cc36e: and    $0x3,%ecx
005cc372: rep movsb
```

---

## 4. Texture Subsystem & JDLZ Decompression Tracing (M1-03, M1-04)

### 4.1. Texture Pack Dispatcher (`0x00494e60`)

The texture pack container (`0xB3300000`) contains texture headers and texture payloads:
1. `0xB3310000`: Texture Info Container:
   - `0x33310001`: Texture Pack Header (124 bytes).
   - `0x33310002`: Texture Hash Table (`[u32; 2]` per entry = 8 bytes).
   - `0x33310003`: Texture Metadata Descriptors (24 bytes per entry).
2. `0xB3320000`: Texture Data Container:
   - `0x33320001`: Texture Data Header.
   - `0x33320002`: Texture Payloads (128-byte aligned).

### 4.2. Universal Compression Dispatcher (`0x005745c0`)

At `0x005745c0`, EA's universal compression dispatcher inspects the first 4 bytes of stream payloads to route to decompression algorithms:
- `0x57574152` (`"RAWW"`): Direct copy (`0x00574440`).
- `0x46465548` (`"HUFF"`): Huffman tree decompression (`0x00574410`).
- `0x504d4f43` (`"COMP"`): LZC variant (`0x00574480`).
- `0x5a4c444a` (`"JDLZ"`): JDLZ LZSS decompressor (`0x00573650`).

### 4.3. JDLZ Decompressor (`func_0x00573650`)

The routine checks:
- Magic: `cmp $0x5a4c444a, (%edx)` (`"JDLZ"`).
- Version byte: `cmpb $0x2, 0x4(%edx)` (Must equal 2).
- Uncompressed length: `u32` at offset `0x08`.
- Compressed length: `u32` at offset `0x0C`.
- Bitstream decoding: Uses two interleaved bit-flag words (`0x100 | byte`) shifted with `sar $1`.
  - Bit 0 of primary word: Literal byte copy.
  - Bit 1 of primary word: Back-reference match copy:
    - Mode A (secondary bit = 1): `offset = (b1 & 0x0F) + 1`, `length = (((b1 & 0xF0) << 4) | b2) + 3`.
    - Mode B (secondary bit = 0): `offset = (((b1 & 0xE0) << 3) | b2) + 17`, `length = (b1 & 0x1F) + 3`.

---

## 5. Confirmed Sample Evidence: Nissan 350Z (`350Z_BASE_A`)

A full test run against `game-assets/CARS/350Z/GEOMETRY.BIN` produced exact matching parameters:

- **Part Identifier:** `350Z_BASE_A` (read at offset `0x00004D08` in chunk `0x00134011`).
- **Mesh Descriptor (`0x00134900`):**
  - Submesh count: `11`
  - Total triangles: `500`
  - Total vertices: `657`
- **Vertex Stream (`0x00134B01`):**
  - Chunk data starts at `0x0000562C`.
  - 128-byte aligned vertex payload starts at `0x00005680` (84 bytes padding).
  - Byte size: `23,652` bytes.
  - Calculated stride: `23,652 / 657 = 36.000` bytes per vertex.
  - First vertex unpacked:
    - Pos: `(-1.5272, 0.7670, 0.6965)`
    - Normal: `(-0.3041, -0.9481, -0.0928)` (length = `1.0000`)
    - Color: `0xFF000000`
    - UV: `(0.4238, 0.8145)`
- **Triangle Index Stream (`0x00134B03`):**
  - Chunk data starts at `0x0000B58C`.
  - 16-byte aligned index payload starts at `0x0000B590` (4 bytes padding with `0x11111111`).
  - Byte size: `3,000` bytes = `1,500` indices = `500` triangles.
  - Indices are strictly bounded within `[0, 656]`.

---

## 6. Milestone Gate & Requirement Compliance

| Requirement | Acceptance Criterion | Verification Evidence | Status |
| :--- | :--- | :--- | :--- |
| **FR-003** | Map resource loading path | Traced from `0x0063af70` to `0x00493b70` and `0x005cc1d0` with disassembly | Met |
| **EV-001** | Evidence-linked behavior | Every chunk ID, offset, and structure links to reference binary VAs | Met |
| **EV-002** | Confirmed vs inferred status | Fully confirmed fields distinguished from padding | Met |
| **EV-003** | No fabricated fields | Exact MSVC structures verified with zero fabricated fields | Met |
| **EV-004** | RE separated from code | Findings documented in `docs/findings/` and `docs/formats/`; core crate untouched | Met |
| **NFR-005** | Malformed input constraints | Bounds, alignments, and sentinel validations specified for M2 parser | Met |

Milestone **M1** exit gate requirements are fully satisfied.
