# Software Requirements Specification — NFSU2 Native Android Engine

- **Version:** 0.1 (draft)
- **Date:** 2026-10-17
- **Status:** Baseline for agent-led reverse engineering and incremental implementation

---

## 1. Purpose and Scope

Build an independent Rust engine that reproduces selected, measurable gameplay behavior of Need for Speed Underground 2 on Android ARM64, using a lawfully obtained PC release as the analysis reference. The initial release is a technical vertical slice, not a promise of a complete playable game.

- **In scope:** binary analysis, documented asset formats, local asset conversion, vehicle and world rendering, basic camera/input, incremental driving simulation, profiling and Android packaging.
- **Out of scope for initial milestones:** multiplayer, backend services, accounts, cloud saves, game-wide content parity, browser build, commercial asset redistribution, anti-cheat and store publication.

---

## 2. Stakeholders and Environment

- **Developer/operator:** runs AGY with REA and Ghidra; reviews findings and generated code.
- **Target user:** runs the Android app and provides legitimately obtained game assets locally, where permitted.
- **Reference:** original PC executable and installation files; precise edition and hashes must be recorded.
- **Initial test device:** POCO X8 Pro Max, 12 GB RAM. This is a development target, not the minimum supported device.

---

## 3. Evidence and Implementation Rules

- **EV-001:** Every reconstructed behavior must link to a reproducible observation, binary address/call graph, asset sample or validated test.
- **EV-002:** Each finding must carry confirmed, inferred or unknown status, plus source hash and tool version.
- **EV-003:** Unknown formats, offsets, APIs and game rules must remain explicitly unknown; the agent must not fabricate them.
- **EV-004:** Reverse-engineering notes and Rust implementation changes must be separate artifacts and separate review steps.
- **EV-005:** No implementation may be called equivalent to the original solely because it compiles or looks plausible.

---

## 4. Functional Requirements

| ID | Requirement | Acceptance criterion | Priority |
| :--- | :--- | :--- | :--- |
| **FR-001** | Inventory source files and establish provenance. | Manifest lists path, size, SHA-256, edition (if known) and source role. | Must |
| **FR-002** | Analyze the reference executable with Ghidra/REA. | Architecture, entry point, at least one function, assembly/pseudocode and references documented with addresses. | Must |
| **FR-003** | Map the resource loading path. | At least one file-opening/reading path traced to a documented asset sample, or a recorded blocker with evidence. | Must |
| **FR-004** | Implement safe parsers for confirmed formats. | Fixtures pass; malformed/truncated input returns typed errors without panic or out-of-bounds access. | Must |
| **FR-005** | Convert supported assets to a versioned runtime representation. | Converter produces deterministic outputs and preserves source-to-output traceability. | Must |
| **FR-006** | Render a vehicle model with its supported materials. | On-device screenshot and profiling capture; visual comparison with reference assets. | Must |
| **FR-007** | Render a bounded world section and camera. | User can navigate a test camera without blocking asset loads on the render thread. | Must |
| **FR-008** | Implement Android input and lifecycle handling. | Touch controls work; pause/resume and surface recreation do not crash or corrupt state. | Must |
| **FR-009** | Implement initial driving simulation from measured reference behavior. | Acceleration, braking and steering scenarios are reproducible; deviations recorded. | Should |
| **FR-010** | Stream and evict runtime assets under memory limits. | Traversing test regions shows bounded cache use and no persistent growth after unload. | Should |
| **FR-011** | Support local import of user-provided assets. | Import validates required files, reports missing/invalid inputs and does not upload them. | Should |
| **FR-012** | Capture diagnostic metrics. | FPS, CPU/GPU frame times (when supported), app memory and loading durations can be exported for analysis. | Must |

---

## 5. Nonfunctional Requirements

| ID | Requirement | Validation |
| :--- | :--- | :--- |
| **NFR-001** | Android ARM64 is the primary runtime target. | Build and launch on physical ARM64 device. |
| **NFR-002** | Initial graphics path uses Vulkan through wgpu, subject to compatibility validation. | Capability probe and rendered smoke test. |
| **NFR-003** | 30 FPS stable at 720p is the baseline target; 60 FPS is aspirational. | Sustained gameplay-like benchmark with frame-time percentiles and thermal state recorded. |
| **NFR-004** | Approx. 1 GiB application memory is an initial budget, not a guaranteed limit. | Peak PSS/RSS and GPU memory where observable, measured on device. |
| **NFR-005** | No untrusted asset may trigger unchecked indexing, integer overflow or uncontrolled allocation. | Parser negative tests, fuzz tests and size limits. |
| **NFR-006** | Engine core must not depend on Android UI classes. | Core crates compile and test on desktop host. |
| **NFR-007** | Code must be formatted, linted, tested and maintainable. | Formatting, Clippy and test gates pass. |
| **NFR-008** | App runs offline after assets are imported. | Airplane-mode smoke test; no runtime backend dependency. |
| **NFR-009** | Commercial game binaries and assets are excluded from repository and release artifacts. | Artifact/repository content audit. |
| **NFR-010** | Logs must avoid dumping proprietary asset contents or sensitive local paths by default. | Logging review and tests where applicable. |

---

## 6. Milestones and Exit Gates

### M0 — Toolchain and source validation
- AGY can invoke REA; Ghidra analyzes the exact PC executable.
- Source manifest, tool versions and reproducible analysis workspace exist.
- **Exit gate:** FR-001, FR-002 and EV-001–003 met.

### M1 — Asset format discovery
- Identify one real vehicle-related asset and its loading path.
- Write a format specification with byte offsets supported by samples.
- **Exit gate:** FR-003 met, parser design reviewed; unknown fields explicitly marked.

### M2 — Safe parser and converter
- Implement a parser for a confirmed format and offline converter.
- Add golden fixtures, corrupt-input tests and format-version checks.
- **Exit gate:** FR-004, FR-005 and NFR-005 met.

### M3 — Android rendering vertical slice
- Display one vehicle with camera and touch interaction.
- Record screenshot, logs, memory and frame times on physical device.
- **Exit gate:** FR-006, FR-008, FR-012 and NFR-001–002 met.

### M4 — World slice and driving
- Render one bounded city region; introduce streaming and measured driving behavior.
- **Exit gate:** FR-007, FR-009, FR-010 with benchmark report; update performance targets from measurements.

### M5 — Compatibility and optimization
- Validate sustained performance and lower-memory device behavior.
- **Exit gate:** NFR-003–004 assessed with measured results; publish actual minimum requirements only after device testing.

---

## 7. Required Project Artifacts

```text
nfsu2-android/
├── docs/
│   ├── adr.md
│   ├── srs.md
│   ├── roadmap.md
│   ├── findings/              # Evidence-backed reverse engineering notes
│   ├── formats/               # Confirmed binary format specifications
│   └── benchmarks/            # Device measurements and comparisons
├── tools/                     # Import/conversion and analysis helpers
├── crates/                    # Rust engine modules (introduced as needed)
├── android/                   # Android host application
└── local-input/               # Gitignored proprietary inputs
```

---

## 8. Agent Execution Contract

1. Read `adr.md` and `srs.md` before proposing code.
2. Work on one milestone and one verifiable vertical slice at a time.
3. For reverse-engineering work, produce a finding with source hash, addresses, call graph or observed sample, confidence level and reproduction steps.
4. Present assumptions and blockers before implementing uncertain behavior.
5. Write tests that validate formats, invariants, error paths or observed game rules; do not add tests merely to increase coverage.
6. Do not change architecture or introduce a dependency without an ADR update proposal.
7. Never modify source binaries; never commit proprietary assets.
8. Finish each task with changed files, executed commands, test results, evidence links and remaining risks. Do not claim tests ran when they did not.

---

## 9. Traceability and Open Questions

- **Traceability:** Every implementation PR must reference at least one FR/NFR, a milestone, and the finding(s) supporting reconstructed behavior.
- **Unresolved before M1:** Exact PC edition and executable hash; REA/Ghidra compatibility; actual asset directory/format inventory; permitted distribution model; confirmed GPU/backend support on the target device; measured memory and frame-time baselines.
- **Change control:** Update this document when measurements or evidence change requirements. Preserve requirement IDs and record rationale for removals or scope changes.
