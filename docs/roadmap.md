# Roadmap — NFSU2 Native Android Engine

- **Version:** 0.1
- **Updated:** 2026-10-17
- **Overall status:** Not started
- **Source of truth:** [adr.md](adr.md) (architectural decisions) and [srs.md](srs.md) (requirements and acceptance criteria).
- **Objective:** Build an evidence-driven Rust engine targeting Android ARM64, using a lawfully obtained, unmodified PC release of Need for Speed Underground 2 as the analysis reference.

---

## 1. Progress Dashboard

| Milestone | Deliverable | Status | Progress | Exit requirements |
| :--- | :--- | :--- | :--- | :--- |
| **M0** | Toolchain and reference executable validated | Done | 6/6 | FR-001, FR-002, EV-001–003 |
| **M1** | First verified vehicle asset format | Not started | 0/5 | FR-003; reviewed parser design |
| **M2** | Safe Rust parser and offline converter | Not started | 0/6 | FR-004, FR-005, NFR-005 |
| **M3** | Textured vehicle running on Android | In progress | 1/7 | FR-006, FR-008, FR-012, NFR-001–002 |
| **M4** | World slice and initial driving | Not started | 0/6 | FR-007, FR-009, FR-010 |
| **M5** | Profiling, optimization and compatibility | Not started | 0/6 | NFR-003–004 assessed with evidence |
| **Total** | | **In progress** | **7/36** | |

> **Status vocabulary:** Not started → In progress → Blocked / In review → Done. A milestone is Done only when its exit gate has verifiable evidence. Checklist completion alone is insufficient.

---

## 2. Milestones and Checklists

### M0 — Toolchain and source validation

**Goal:** Prove that AGY can analyze the exact PC executable through REA and Ghidra.

- [x] **M0-01** — Install and record versions of Rust, Android SDK/NDK, JDK, Ghidra, Node.js, REA and AGY.
- [x] **M0-02** — Configure AGY ↔ REA ↔ Ghidra; execute a real integration smoke test.
- [x] **M0-03** — Obtain an unmodified, lawfully sourced PC installation; confirm the actual executable filename and edition.
- [x] **M0-04** — Generate an inventory with paths, sizes, SHA-256 and provenance; keep source inputs gitignored.
- [x] **M0-05** — Confirm x86 architecture, entry point, at least one function, assembly/pseudocode and cross-references.
- [x] **M0-06** — Record findings with addresses, hashes, tool versions, confidence and reproducible steps.

- **Evidence:** `docs/findings/M0-toolchain.md`, `docs/findings/M0-executable.md`, local manifest (without proprietary file contents).
- **Exit gate:** FR-001, FR-002 and EV-001–003 demonstrated. If REA cannot analyze the executable, mark Blocked, record diagnostics and propose a reviewed fallback; do not silently change the reference platform.

---

### M1 — Asset format discovery

**Goal:** Identify a real vehicle-related asset and trace how the original game reads it.

- [ ] **M1-01** — Inventory the real installation tree; distinguish observed filenames from hypotheses.
- [ ] **M1-02** — Identify at least one candidate vehicle asset and its container/format, if any.
- [ ] **M1-03** — Trace a file-open/read/parse path using addresses and references in Ghidra/REA.
- [ ] **M1-04** — Document confirmed fields, offsets, byte order, compression and unresolved fields with sample evidence.
- [ ] **M1-05** — Review a minimal Rust parser interface and malformed-input constraints before implementation.

- **Evidence:** `docs/findings/M1-loading-path.md`, `docs/formats/<confirmed-format>.md`.
- **Exit gate:** FR-003 demonstrated with a verified sample and reviewed parser design. If no format can be confirmed, stop and document the blocker rather than fabricate a schema.

---

### M2 — Safe parser and offline converter

**Goal:** Convert one confirmed asset format into a deterministic, versioned runtime representation.

- [ ] **M2-01** — Implement a bounded, typed Rust parser for the verified format.
- [ ] **M2-02** — Add golden-file tests validating known fields and values.
- [ ] **M2-03** — Add negative tests for truncation, invalid offsets, sizes and integer overflow.
- [ ] **M2-04** — Implement an offline converter with explicit input/output format versions.
- [ ] **M2-05** — Verify deterministic conversion and traceability to source hashes.
- [ ] **M2-06** — Run formatting, Clippy and tests; document exact commands and results.

- **Evidence:** parser tests, fixture provenance, converter report, `docs/findings/M2-conversion.md`.
- **Exit gate:** FR-004, FR-005 and NFR-005 demonstrated. Do not commit commercial fixture contents; use legally shareable synthetic fixtures in the repository and local originals for private verification.

---

### M3 — Android rendering vertical slice

**Goal:** Display one textured vehicle on the physical Android test device.

- [x] **M3-01** — Create a minimal Cargo workspace and Android ARM64 host; keep platform-specific code isolated.
- [ ] **M3-02** — Probe Vulkan/wgpu capabilities on the target device and record the result.
- [ ] **M3-03** — Render the converted vehicle geometry and supported textures/materials.
- [ ] **M3-04** — Implement a movable camera and basic touch input.
- [ ] **M3-05** — Handle pause/resume and surface recreation safely.
- [ ] **M3-06** — Capture screenshot, logs, FPS/frame times and application memory on device.
- [ ] **M3-07** — Execute Android smoke tests and document results and limitations.

- **Evidence:** `docs/benchmarks/M3-device-baseline.md`, screenshot reference, build/test logs.
- **Exit gate:** FR-006, FR-008, FR-012 and NFR-001–002 demonstrated. Target device: user-provided POCO X8 Pro Max, 12 GB RAM; exact chipset/GPU must be measured rather than assumed.

---

### M4 — World slice and initial driving

**Goal:** Render a bounded section of the world and drive a vehicle with behavior grounded in reference measurements.

- [ ] **M4-01** — Discover and document required world geometry and collision data formats.
- [ ] **M4-02** — Render one bounded region with camera movement and visibility culling.
- [ ] **M4-03** — Implement bounded asynchronous asset loading and unloading; verify no persistent growth.
- [ ] **M4-04** — Measure reference acceleration, braking and steering in reproducible scenarios.
- [ ] **M4-05** — Implement fixed-step vehicle simulation and compare against reference measurements.
- [ ] **M4-06** — Capture a representative device benchmark and document deviations/blockers.

- **Evidence:** `docs/findings/M4-driving-reference.md`, `docs/benchmarks/M4-world-slice.md`.
- **Exit gate:** FR-007, FR-009 and FR-010 demonstrated. Do not claim physics equivalence without comparison data.

---

### M5 — Compatibility and optimization

**Goal:** Establish measured performance and supported device requirements, not speculative ones.

- [ ] **M5-01** — Profile CPU/GPU frame times, memory, loading and stalls on the primary device.
- [ ] **M5-02** — Run a sustained 20–30 minute test and record thermal effects.
- [ ] **M5-03** — Prioritize optimizations from profiling (LOD, culling, texture compression, streaming, batching as applicable).
- [ ] **M5-04** — Re-run benchmarks after each optimization and record before/after results.
- [ ] **M5-05** — Test at least one lower-memory Android device before setting minimum requirements.
- [ ] **M5-06** — Audit offline operation, malformed assets, licensing, packaging and repository contents.

- **Evidence:** `docs/benchmarks/M5-performance.md`, device compatibility matrix, release-readiness review.
- **Exit gate:** NFR-003–004 assessed against real measurements; 30 FPS at 720p is an initial goal, 60 FPS is a stretch goal, and ~1 GiB app memory is a provisional budget, not a guarantee.

---

## 3. Execution Rules for AGY

1. **At task start:** read `docs/adr.md`, `docs/srs.md` and `docs/roadmap.md`; select only the next eligible unchecked task.
2. **Before implementation:** identify requirement IDs, existing evidence and testable acceptance criteria. If evidence is missing, investigate instead of guessing.
3. **During work:** keep reverse-engineering findings separate from production Rust code; do not modify original binaries or commit proprietary assets.
4. **After work:** run relevant tests and record commands, outcomes, changed files and evidence paths. Mark a checkbox only after verification.
5. **Milestone gate:** update its status to In review when all tasks are checked; set Done only after exit requirements are independently validated.
6. **Blockers:** set status Blocked, record the precise failure, attempts, impact and proposed options. Do not mark incomplete work as complete.
7. **Architecture changes:** propose an ADR amendment before changing accepted decisions or adding major dependencies.
8. **Progress reporting:** update the dashboard counts and the execution log in the same change as task status. Never fabricate benchmark results or test execution.

---

## 4. Work Item Record Template

Use this template for meaningful tasks in `docs/findings/` or the associated pull request:

```markdown
### <Task ID> — <Title>
- **Status:** Not started | In progress | Blocked | In review | Done
- **Requirements:** FR-xxx / NFR-xxx / EV-xxx
- **Source evidence:** <hash, binary address, sample or measured observation>
- **Implementation:** <paths and summary>
- **Validation:** <exact commands and observed results>
- **Risks/blockers:** <facts and next action>
- **Reviewed by:** <reviewer/date, if applicable>
```

---

## 5. Execution Log

| Date | Task | Change | Evidence | Result |
| :--- | :--- | :--- | :--- | :--- |
| 2026-10-17 | Planning | Initial roadmap aligned with ADR v0.1 and SRS v0.1 | `docs/adr.md`, `docs/srs.md` | No engineering tasks started |
| 2026-10-17 | M0-01, M0-02 | Toolchain inventoried, cargo-ndk installed, REA/Ghidra configured and verified via live smoke test | `docs/findings/M0-toolchain.md` | Verified (2/6 tasks Done) |
| 2026-10-17 | M3-01 | Minimal Cargo workspace and Android ARM64 host crate initialized with platform isolation traits | `crates/core`, `crates/android-host` | Verified (8 unit tests, clippy, aarch64 target build pass) |
| 2026-10-17 | M0-03..M0-06 | Sourced PC installation verified; 1,591 assets inventoried; SPEED2.EXE analyzed via Ghidra/REA (entry, WinMain, CreateFileA file loading paths decompiled) | `docs/findings/M0-executable.md`, `docs/findings/M0-manifest.md`, `reverse-engineering/extraction/README.md` | Verified (M0 Done: 6/6) |

---

## 6. Current Next Action

**M1-01 & M1-02:** Select candidate vehicle asset `CARS/350Z/GEOMETRY.BIN` and `TEXTURES.BIN`; trace loading path in `SPEED2.EXE` via Ghidra/REA.
