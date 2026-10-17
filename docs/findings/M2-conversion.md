# M2 Finding: Safe Parser, Runtime Format (.onfs-model), and Deterministic Converter

**Date:** 2026-10-17  
**Milestone:** M2 — Safe Parser and Offline Converter  
**Scope:** `crates/asset`, `crates/asset-runtime`, `tools/converter`  
**Status:** Complete & Verified  

---

## 1. Executive Summary

Milestone M2 establishes the safe asset pipeline for `open-nfs`, bridging legacy Need for Speed Underground 2 PC asset containers (`GEOMETRY.BIN`, `TEXTURES.BIN`) and modern Android Vulkan/wgpu execution.

Key deliverables:
1. **Safe Zero-Panic Parser (`crates/asset`)**:
   - Strictly bounded chunk parsing (`ChunkIterator`) protecting against `0xFFFFFFFF` overflow and malformed headers.
   - Decompiled EA JDLZ v2 LZSS decoder (`decompress_jdlz`) with bounds checking on back-references.
   - Strict `#![deny(unsafe_code)]` compliance and zero `.unwrap()` / `.expect()` calls on untrusted input.
2. **Runtime Model Format (`crates/asset-runtime`)**:
   - `.onfs-model` v1 specification (`ONFM_MAGIC = b"ONFM"`, version `1`).
   - Cache-aligned vertex layout (`RuntimeVertex`, 36 bytes) deriving `bytemuck::Pod` and `bytemuck::Zeroable` for direct mobile GPU staging uploads.
   - **Dual-Mode Asset Loader**: Supports both `AssetSource::Embedded(&[u8])` (for clean APK bundled assets) and `AssetSource::ExternalPath(&Path)` (for user local storage / SD card import).
3. **Offline Converter Tool (`tools/converter`)**:
   - `open-nfs-converter` CLI tool supporting `--bundle-dir` and `--manifest`.
   - 100% bit-for-bit deterministic output verified against reference `game-assets/CARS/350Z/GEOMETRY.BIN`.
   - 26.5% file size reduction (from 7.56 MiB down to 5.56 MiB) by stripping EA legacy padding while preserving exact vertex attributes, normals, UVs, and triangle meshes.

---

## 2. Technical Findings & Corrections

### 2.1. Part Header (`0x00134011`) Structure Correction
During conversion testing across all 607 car parts in the Nissan 350Z geometry archive, our initial hypothesis that the ASCII name was located at `0x40..0x60` was disproven by binary inspection:
- `0x40..0x7F` (`64 bytes`): Contains the 4x4 local transform / orientation matrix (`[1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1]` identity matrix in primary body parts).
- `0x80..0xA3` (`36 bytes`): Contains mount vectors and animation parameters.
- `0xA4..0xBF` (`28 bytes`): Confirmed location of the null-terminated ASCII part name (e.g. `"350Z_BASE_A\0"`, `"350Z_STYLE01_HOOD_A\0"`).

### 2.2. Dual Vertex Stride Discovery: 36-Byte (Lit) vs 24-Byte (Unlit)
During garage-wide batch conversion across all vehicles in `game-assets/CARS/`, vehicle `3000GT_KIT00_ENGINE_B` revealed a secondary vertex stride:
- **36-byte stride (`0xC180`)**: Used for all standard exterior/interior body parts (`pos: [f32; 3]` [12 bytes], `normal: [f32; 3]` [12 bytes], `color: u32` [4 bytes], `uv: [f32; 2]` [8 bytes]).
- **24-byte stride (`0x0000`)**: Used for unlit internal sub-components (such as engine bay blocks and undercarriages) omitting the normal vector (`pos: [f32; 3]` [12 bytes], `color: u32` [4 bytes], `uv: [f32; 2]` [8 bytes]).

The parser dynamically checks `desc_vcount * 24 == vert_bytes.len()` and populates standard unit normals (`[0.0, 1.0, 0.0]`) to maintain a unified 36-byte `RuntimeVertex` layout for mobile GPU vertex buffers.

---

## 3. Reference Asset Conversion & Verification Data

### 3.1. Single Reference Asset (Nissan 350Z)

| Metric | Original PC Binary (`GEOMETRY.BIN`) | Converted Runtime Model (`.onfs-model`) |
| :--- | :--- | :--- |
| **File Path** | `game-assets/CARS/350Z/GEOMETRY.BIN` | `/tmp/350Z.onfs-model` |
| **File Size** | 7,930,864 bytes (7.56 MiB) | 5,829,589 bytes (5.56 MiB) |
| **Size Ratio** | 100.0% | **73.5% (26.5% reduction)** |
| **SHA-256 Hash** | `90b66874fdf16b7bafdd760cdcfcc412bae4eb584b0bbbb755d2569617c57650` | `314d77349e78a2293dd0f598a03dbfcc1263ff52f2c8480fa37b0a36e401bca2` |
| **Total Parts** | 607 parts | 607 parts |
| **Total Vertices** | 140,132 vertices | 140,132 vertices |
| **Total Indices** | 329,448 indices | 329,448 indices |

### 3.2. Full Garage Batch Conversion Benchmark (All Game Vehicles)
Batch command:
```bash
open-nfs-converter --batch-dir game-assets/CARS/ --output-dir /tmp/converted-cars/ --manifest /tmp/all-cars-manifest.json
```

| Metric | Full Game PC Assets (`game-assets/CARS/*`) | Converted Mobile Runtime Garage |
| :--- | :--- | :--- |
| **Total Vehicles / Models** | 56 containers | **56 `.onfs-model` files** |
| **Total Parts** | 18,828 parts | **18,828 parts** |
| **Total Vertices** | 4,111,663 vertices | **4,111,663 vertices** |
| **Total Triangle Indices** | 9,607,000 indices | **9,607,000 indices** |
| **Total Disk Storage** | 220.45 MiB (231,158,256 bytes) | **163.18 MiB (171,110,197 bytes)** |
| **Overall Size Savings** | — | **26.0% storage reduction** |
| **Batch Conversion Time** | — | **~1.5 seconds** |

### Deterministic Reproducibility
```bash
open-nfs-converter --input game-assets/CARS/350Z/GEOMETRY.BIN --output /tmp/350Z.onfs-model
open-nfs-converter --input game-assets/CARS/350Z/GEOMETRY.BIN --output /tmp/350Z-repeat.onfs-model
cmp /tmp/350Z.onfs-model /tmp/350Z-repeat.onfs-model
# Output: Exit code 0 (Bit-for-bit identical)
```

---

## 4. Test Suite Summary

- **Total Test Cases**: 28 passing tests across workspace.
  - `open-nfs-core`: 5 tests (math POD, vector operations, chunk ID, fixed delta, RPM).
  - `open-nfs-android-host`: 3 tests (lifecycle transitions, fixed-step tick, touch input).
  - `open-nfs-asset` (golden): 2 tests (synthetic cube golden parse, local 350Z asset verification).
  - `open-nfs-asset` (negative): 9 tests (empty buffer, truncated headers, invalid magic, oversized chunk size, back-ref out of bounds).
  - `open-nfs-asset-runtime`: 7 tests (roundtrip serialize/deserialize, embedded loader, external path loader, bad magic, bad version, out of bounds index, loading converted 350Z model).
  - `open-nfs-converter`: 2 tests (FIPS 180-4 SHA-256 test vectors).
- **Quality Gates**:
  - `cargo fmt --all -- --check`: Passed (zero warnings).
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`: Passed (zero warnings).
  - `cargo test --workspace`: 28 passed, 0 failed.
