# AGENTS.md — Engineering Guidelines & Rust Operating Standards

This document establishes the mandatory engineering standards, Rust programming paradigms, and operational workflow for AI agents (AGY, Claude, Gemini, and contributors) developing the **NFSU2 Native Android Engine** (`open-nfs`).

---

## 1. Project Foundations & Non-Negotiables

All agents operating in this repository must align with the foundational requirements and decisions documented in [`docs/adr.md`](docs/adr.md), [`docs/srs.md`](docs/srs.md), and [`docs/roadmap.md`](docs/roadmap.md):

1. **Evidence-Driven Reverse Engineering (EV-001–005)**:
   - Never hypothesize or hallucinate binary structures, file formats, offsets, or physics formulas.
   - Every format parser or reconstructed algorithm must trace directly to an address, call graph, sample fixture, or reproducible measurement recorded in `docs/findings/`.
   - Unknown fields must be preserved and explicitly documented as `unknown_0x...` or padding rather than guessed.

2. **Proprietary Asset Exclusion (ADR-008, NFR-009)**:
   - **Never** commit original PC executables (`SPEED2.EXE`), CD images, commercial textures, geometries, or audio assets to version control.
   - Tests must run on synthetic, procedurally generated, or legally shareable fixtures. Original game assets remain strictly local in gitignored directories (`local-input/`).

3. **Platform Independence of Engine Core (ADR-002, NFR-006)**:
   - Target runtime is Android ARM64 (`aarch64-linux-android`), but the engine core crates (`crates/core`, `crates/asset`, `crates/sim`, `crates/render`) **must compile, run, and be testable on host desktop platforms** (`x86_64-unknown-linux-gnu`).
   - Android-specific APIs (NDK, JNI, `android-activity`, surface lifecycle) must be isolated inside `crates/android-host` behind platform-agnostic traits.

---

## 2. Rust Architectural & Coding Paradigms

### 2.1. Type-Driven Design & Invariant Modeling

- **Parse, Don't Validate**: Validate untrusted or raw binary data at the boundary. Once parsed, convert it into strongly typed structures that make invalid states unrepresentable.
- **Newtype Pattern**: Avoid primitive obsession. Wrap domain primitives in newtypes to prevent semantic errors:
  ```rust
  // Good: Prevents mixing up coordinate systems or units
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
  pub struct ChunkId(pub u32);

  #[derive(Debug, Clone, Copy, PartialEq)]
  pub struct Rpm(pub f32);

  #[derive(Debug, Clone, Copy, PartialEq)]
  pub struct FixedDelta(pub std::time::Duration);
  ```
- **Typestate Pattern**: Model system or asset lifecycles via the type system rather than runtime boolean flags:
  ```rust
  pub struct AssetHandle<S: AssetState> {
      id: AssetId,
      state: S,
  }

  pub struct Unloaded;
  pub struct Loaded {
      data: Arc<RuntimeGeometry>,
  }

  impl AssetHandle<Unloaded> {
      pub fn load(self, device: &wgpu::Device) -> Result<AssetHandle<Loaded>, AssetError> {
          // ...
      }
  }
  ```
- **Exhaustive Pattern Matching**: Avoid wildcard `_ =>` patterns in matches over domain enums. If a new variant is introduced, the compiler must force the developer to consider every call site.

---

### 2.2. Memory Management & Real-Time Allocation Discipline

In an engine targeting 30–60 FPS on mobile GPUs with a ~1 GiB RAM budget (NFR-003, NFR-004):

1. **Zero-Allocation Hot Paths**:
   - **Frame loop, render pass, and fixed physics tick must not perform dynamic heap allocations (`Vec::new`, `Box`, `String`, etc.)**.
   - Pre-allocate staging buffers, frame-scratch vectors, and instance buffers during initialization or resize events.
   - Clear collections with `.clear()` to retain pre-allocated capacities instead of re-instantiating.
2. **Arena & Pool Allocation**:
   - Use arena/bump allocators (e.g., `bumpalo`) for transient per-frame scratch memories that can be reset all at once at the end of a frame.
   - Use object pools or slotmaps (e.g., `slotmap`) for entities and particle arrays with stable IDs.
3. **Data-Oriented Layout (DoD)**:
   - Favor contiguous array buffers (`Vec<T>` with `#[repr(C)]`) over pointer-chasing structures (`Vec<Box<dyn Trait>>` or nested tree allocations).
   - Ensure vertex, matrix, and transform buffers are cache-aligned and compatible with GPU staging uploads (`bytemuck::Pod` and `bytemuck::Zeroable`).

---

### 2.3. Robust Binary Parsing & Asset Conversion (ADR-004, NFR-005)

When reading legacy PC file formats:

- **Safe Binary Deserialization**:
  - Always enforce bounded reads. Check byte slices against explicit buffer lengths before reading.
  - Never trust raw length or count fields read from file headers without bounds checking:
    ```rust
    // Reject pathological counts that could cause integer overflow or OOM
    let count = reader.read_u32_le()? as usize;
    if count > MAX_ALLOWED_CHUNKS || count * std::mem::size_of::<ChunkHeader>() > remaining_bytes {
        return Err(ParseError::InvalidChunkCount(count));
    }
    ```
  - Use `checked_add`, `checked_mul`, and `checked_sub` when computing byte offsets from file data.
- **Explicit Endianness**:
  - Never use native endianness assumptions. Always specify `.read_u32_le()`, `u32::from_le_bytes`, or `.read_u32_be()`.
- **Zero-Copy with Safety Guarantees**:
  - For large vertex/index arrays, prefer zero-copy casting with vetted crates like `bytemuck` over manual pointer casting. Structures must derive `bytemuck::Pod` and `bytemuck::Zeroable` with explicit `#[repr(C)]`.

---

### 2.4. Error Handling & Panic Prevention

- **Libraries & Engine Crates (`crates/*`)**:
  - **Zero Panics on Untrusted Input**: Never call `.unwrap()` or `.expect()` on asset parsing, network, or file I/O operations.
  - Use [`thiserror`](https://docs.rs/thiserror) for explicit, typed error enums per crate:
    ```rust
    #[derive(thiserror::Error, Debug)]
    pub enum FormatError {
        #[error("Magic mismatch: expected {expected:?}, found {found:?}")]
        InvalidMagic { expected: [u8; 4], found: [u8; 4] },
        #[error("Unexpected EOF while reading {section} at offset {offset:#x}")]
        UnexpectedEof { section: &'static str, offset: usize },
        #[error("Unsupported format version: {0}")]
        UnsupportedVersion(u32),
    }
    ```
  - Propagate errors with `?`.
- **CLI & Analysis Tools (`tools/*`)**:
  - Use `anyhow` or `miette` for rich human-facing error reports with context.
- **Degraded Execution vs. Crashing**:
  - A corrupted car livery texture or missing sound effect must log a warning and fallback to a default magenta checkerboard / silence, rather than crashing the engine.
  - Core invariant violations in debug builds should use `debug_assert!` to catch bugs early without hurting release performance.

---

### 2.5. Concurrency & Data Flow Architecture

- **Thread Separation**:
  1. **Render Thread**: Runs the wgpu rendering pass and frame presentation pacing.
  2. **Simulation Thread**: Fixed-timestep loop (e.g., 60 Hz or 120 Hz) for vehicle dynamics and kinematics (ADR-005).
  3. **Background IO / Streaming Worker**: Asynchronous streaming for asset loading and decompression.
- **Communication**:
  - Communicate across threads using bounded channels (e.g., `crossbeam-channel` or `flume`) or double-buffered / lock-free ring buffers.
  - Avoid heavy locking (`std::sync::Mutex` or `RwLock` held across async or render boundaries). Keep locks tight, or use message passing.
- **No Global Mutable Singletons**:
  - Do not use global mutable state (`lazy_static!` / static mut). Pass explicit contexts (`Context`, `Engine`, `World`) down the call stack.

---

### 2.6. `unsafe` Code Policy

- The repository enforces `#![deny(unsafe_code)]` across all pure logic, simulation, and format parsing crates.
- `unsafe` is permitted **only** when strictly required:
  - FFI interop with the Android NDK, JNI, or raw Vulkan handles.
  - High-performance SIMD intrinsics where the compiler auto-vectorizer falls short.
- **Rules for Unsafe**:
  1. Every `unsafe` block must be accompanied by an explanatory comment starting with `// SAFETY:` explaining why the invariants cannot be violated.
  2. Encapsulate unsafe blocks into the smallest possible safe abstraction functions.
  3. Validate unsafe functions with unit tests, Miri (`cargo miri test`), and address sanitizers.

---

## 3. Workspace & Code Layout

Organize code according to ADR-007 modularity guidelines:

```text
nfsu2-android/
├── Cargo.toml                 # Workspace manifest
├── docs/                      # Architectural specifications and findings
│   ├── adr.md                 # Architectural Decision Records
│   ├── srs.md                 # Software Requirements Specification
│   ├── roadmap.md             # Milestone roadmap and progress log
│   ├── findings/              # Evidence-backed reverse engineering findings
│   └── formats/               # Confirmed binary format specifications
├── crates/                    # Modular engine crates
│   ├── core/                  # Math, time, memory allocators, common types
│   ├── asset/                 # Safe parsers for confirmed original formats
│   ├── asset-runtime/         # Converted versioned mobile runtime assets
│   ├── sim/                   # Fixed-step vehicle physics and kinematics
│   ├── render/                # wgpu Vulkan graphics backend and pipelines
│   └── android-host/          # Android NDK activity, touch input, lifecycle
├── tools/                     # Desktop offline tools
│   ├── converter/             # Offline converter from PC formats to runtime format
│   └── inspector/             # Asset format debugging and visual inspection
└── local-input/               # (Gitignored) Reference game files and dumps
```

---

## 4. Testing, Verification & Tooling Standards

Every pull request or task execution must satisfy these automated gates:

1. **Formatting**:
   ```bash
   cargo fmt --all -- --check
   ```
   Zero tolerance for unformatted code.

2. **Linting (Clippy)**:
   ```bash
   cargo clippy --workspace --all-targets --all-features -- -D warnings
   ```
   All warnings are treated as errors.

3. **Testing**:
   ```bash
   cargo test --workspace
   ```
   - **Golden File Tests**: Formats must have regression tests validating against known deterministic hashes or fixture outputs.
   - **Negative Tests**: Binary parsers must be tested against truncated, oversized, zero-length, and fuzz-mutated inputs to ensure graceful error handling without panics.
   - **Deterministic Simulation Tests**: Fixed-step physics updates must produce identical results across successive runs given identical initial states and inputs.

---

## 5. Agent Operational Protocol

When assigned a work item:

1. **Orientation**:
   - Check `docs/roadmap.md` for current milestone and task priorities.
   - Review relevant ADRs (`docs/adr.md`) and requirements (`docs/srs.md`).
2. **Investigation & Design**:
   - For reverse engineering tasks, produce notes in `docs/findings/` with tool versions, executable hashes, and binary addresses.
   - Never commit speculative implementations without evidence.
3. **Implementation**:
   - Write clean, modular, documented Rust code adhering to the standards in this guide.
   - Keep crate boundaries well-defined with clear visibility (`pub(crate)` over `pub`).
4. **Verification**:
   - Run `cargo fmt`, `cargo clippy`, and `cargo test`.
   - Record exact commands and output in the task log.
5. **Progress Record**:
   - Update `docs/roadmap.md` (checklist checkbox, Progress Dashboard counts, and Execution Log entry).
