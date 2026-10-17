# M0-04 — Game Assets Inventory & Integrity Manifest

- **Task ID:** M0-04
- **Milestone:** M0 (Toolchain & Reference Executable Validation)
- **Status:** Done
- **Requirements:** FR-001, EV-001, EV-002, ADR-008, NFR-009
- **Date:** 2026-10-17
- **Full Inventory File:** Generated at `docs/findings/M0-manifest.tsv` (1,591 files indexed)

---

## 1. Asset Breakdown by Category & Format

| Asset Group | Primary Extensions | File Count | Approximate Size | Role & Architecture Notes |
| :--- | :--- | :--- | :--- | :--- |
| **Vehicle Assets (`CARS/`)** | `.BIN`, `.bin`, `.gin` | 765 | ~850 MiB | Vehicle meshes (`GEOMETRY.BIN`), vinyls (`VINYLS.BIN`), textures (`TEXTURES.BIN`), animations. |
| **Track Streaming (`TRACKS/`)** | `.BUN`, `.bun` | 67 | ~620 MiB | World geometry bundles (`STREAML4RD.BUN`), collision, and track-specific assets. |
| **Global Resources (`GLOBAL/`)** | `.lzc`, `.bin` | 15 | ~45 MiB | Shared palettes, common textures, and gameplay configuration (`GlobalB.lzc`). |
| **Audio Subsystem (`SOUND/`)** | `.abk`, `.mus`, `.mpf` | 185 | ~320 MiB | Interactive music, sound effects, engine sounds, turbo, and exhaust. |
| **Speech Subsystem (`SPEECH/`)** | `.big`, `.idx`, `.csi`, `.evt`| 8 | ~180 MiB | Police radio, narrative voice lines, and NIS dialogue (`UG2_Speech.big`, `nisaudio.big`). |
| **Video Cutscenes (`MOVIES/`)** | `.vp6` | 29 | ~190 MiB | Electronic Arts VP6 intro and career cutscenes. |
| **User Interface (`FRONTEND/`)** | `.bin`, `.bun`, `.lzc` | 22 | ~35 MiB | Menus, HUD overlays, speedometer, vinyl previews. |
| **Localization (`LANGUAGES/`)** | `.bin`, `.loc`, `.lng` | 38 | ~12 MiB | Multi-language strings (`English.bin`, `agree.eng`, etc.). |
| **Game Executables & Binaries** | `.EXE`, `.exe`, `.dll` | 8 | ~9.5 MiB | `SPEED2.EXE` (v1.2, 4.78 MB), `server.dll`, `dimap.dll`. |
| **Total Tracked** | | **1,591** | **~2.26 GiB** | Fully inventoried with SHA-256 and byte size. |

---

## 2. Key Format Verification Samples

### Vehicle Geometry Sample (`CARS/350Z`)
- `game-assets/CARS/350Z/GEOMETRY.BIN`
  - Size: `7,930,864` bytes
  - SHA-256: `90b66874fdf16b7bafdd760cdcfcc412bae4eb584b0bbbb755d2569617c57650`
  - Target for Milestone M1 format discovery.

### Vehicle Texture Sample (`CARS/350Z`)
- `game-assets/CARS/350Z/TEXTURES.BIN`
  - Size: `617,280` bytes
  - SHA-256: `b78af1e132612468439d97dd494ae7253778be559466ceb1f43dde42040deb68`

### Track Stream Sample (`TRACKS/`)
- `game-assets/TRACKS/STREAML4RD.BUN`
  - Size: `23,592,960` bytes
  - SHA-256: `3a42eb6fc7e02e861d84a77feef9bb57b290cb6bf328eb922c2bf3b185b01ce7`

### Speech Container Sample (`SPEECH/`)
- `game-assets/SPEECH/UG2_Speech.big`
  - Size: `108,658,688` bytes
  - SHA-256: `c8303f26938995a9d3000b9a67448dff90e5f5f7dc537b0d91d6f4625b161247`

---

## 3. Exclusion and Compliance Verification (NFR-009, ADR-008)

All 1,591 commercial files are located exclusively in gitignored directories:
- `/local-input/` (gitignored by pattern `/local-input/*`, except `README.md`)
- `/game-assets/` (gitignored explicitly)
- `/reverse-engineering/binaries/` (gitignored explicitly)

Running `git status` confirms zero commercial assets or proprietary binaries are staged or tracked in git.
