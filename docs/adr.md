# Architecture Decision Records — NFSU2 Android

- **Status:** Proposed
- **Version:** 0.1
- **Date:** 2026-10-17
- **Scope:** Independent, native Android engine informed by analysis of a lawfully obtained PC copy of Need for Speed Underground 2. This is not an assertion of rights to distribute the original game or its assets.

---

## ADR-001 — Reference binary and evidence-based reverse engineering

- **Status:** Accepted for initial implementation.
- **Context:** The project needs an analyzable reference implementation to reconstruct observable behavior. The PS2 binary introduces platform-specific MIPS/R5900 complexity.
- **Decision:** Use the original, unmodified Windows PC executable (`SPEED2.EXE`, subject to verification) as the primary behavioral reference. Analyze with Ghidra and REA through the selected agent environment. Retain the PS2 version only as an optional comparison source; it is not a dependency.
- **Consequences:** x86 decompilation is better supported, but Windows APIs and PC asset formats still require investigation. Never treat decompiler output as verified source code.
- **Verification:** Record file hashes, provenance, architecture, entry point, tool versions, addresses, cross-references and confidence for every finding. Do not commit commercial binaries.

---

## ADR-002 — Independent Rust engine targeting Android ARM64

- **Status:** Accepted.
- **Context:** Direct execution of the Windows binary is not a native Android port.
- **Decision:** Reimplement only behavior supported by evidence in Rust; compile to `aarch64-linux-android`. Keep core simulation and data formats platform-independent. Use a thin Android host layer for lifecycle, input, audio and surface integration.
- **Consequences:** Significant implementation effort; no automatic conversion from REA/Ghidra. Isolate platform code behind traits/interfaces; prefer safe Rust and document any unsafe blocks.

---

## ADR-003 — Rendering and frame pacing

- **Status:** Proposed; validate on device.
- **Context:** The Android target requires efficient graphics on mobile GPUs.
- **Decision:** Prototype wgpu with Vulkan backend. Keep rendering separate from simulation, use fixed-step simulation and presentation pacing, and measure CPU/GPU frame times before optimization. Avoid dependence on a specific GPU vendor.
- **Alternatives:** Direct Vulkan via `ash`; consider only if profiling demonstrates an unsolved wgpu limitation.
- **Verification:** Render a textured vehicle and a small world section on a physical Android device. Record backend availability, GPU model, frame time and thermal behavior.

---

## ADR-004 — Offline asset conversion and runtime streaming

- **Status:** Accepted as architectural direction; source formats unknown.
- **Context:** Original PC resource formats and packing must be established through analysis. Runtime parsing/conversion may cause stalls.
- **Decision:** Separate source-format parsers from an offline conversion pipeline and a versioned, mobile-oriented runtime asset format. Load regions on demand with bounded caches and asynchronous I/O. Preserve an auditable mapping from source asset to converted output.
- **Consequences:** Additional tooling and storage, but predictable runtime behavior. No source-format names, offsets or compression algorithms may be assumed without evidence.
- **Verification:** Round-trip or golden-file tests for each discovered format; memory and streaming measurements on device.

---

## ADR-005 — Gameplay fidelity before replacement physics

- **Status:** Accepted.
- **Context:** A generic physics engine would not necessarily reproduce the original vehicle handling.
- **Decision:** Treat observed game behavior as the reference. Reconstruct driving mechanics incrementally, with deterministic fixed-step tests where feasible. Introduce third-party physics only for isolated functionality after equivalence testing.
- **Verification:** Document reproducible reference scenarios and compare acceleration, braking, steering and collision outcomes within tolerances defined after measurement.

---

## ADR-006 — Performance targets and compatibility

- **Status:** Proposed targets, not measured requirements.
- **Decision:** Primary development device: POCO X8 Pro Max with 12 GB RAM (user-provided). Target Android ARM64, initial 720p internal resolution, stable 30 FPS baseline and 60 FPS stretch goal. Set an initial application memory budget of approximately 1 GiB, to be revised from measurements. Test on lower-memory devices before declaring minimum specifications.
- **Verification:** Profile cold start, sustained 20–30 minute sessions, memory peak, frame-time percentiles, GPU load and thermal throttling. Do not equate 12 GB physical RAM with available app memory.

---

## ADR-007 — Modularity and dependency policy

- **Status:** Accepted.
- **Decision:** Use a Cargo workspace with independent modules for core, assets, simulation, render, audio, and android-host (names may evolve). Dependencies require an explicit use case, license review and version pinning. Prefer bounded data structures, typed errors, structured logs and testable interfaces. Do not introduce an ECS, distributed backend or complex plugin system without demonstrated need.
- **Verification:** `cargo fmt --check`, `cargo clippy -- -D warnings`, unit tests, parser fuzzing where appropriate, and Android smoke tests in CI or documented local runs.

---

## ADR-008 — Rights, security and provenance

- **Status:** Accepted.
- **Decision:** Do not distribute original executables, commercial assets, keys or proprietary game code. Keep source inputs outside version control. Design a local user-provided asset import/conversion workflow; distribution and reverse-engineering legality require jurisdiction-specific review. Treat all game files as untrusted input, with bounds checks, resource limits and no privileged execution. Avoid copying decompiler-generated code verbatim into published source without legal review.
- **Verification:** Repository scans for forbidden binary artifacts; negative tests for malformed assets; documented provenance and license inventory.

---

## Decision Review Triggers

Revisit the relevant ADR when:
- A Ghidra/REA assumption fails;
- An asset format is confirmed;
- Vulkan is unsupported on a target device;
- Measurements invalidate performance budgets; or
- Legal review changes the permissible distribution model.
