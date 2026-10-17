# NFSU2 Asset Extraction & Reverse Engineering Preparation

This directory documents the extraction and preparation of Need for Speed Underground 2 reference files for analysis and engine reconstruction.

---

## 1. Source Package Details

- **Archive / Installer:** `local-input/SPEED2.exe`
- **File Size:** `1,855,301,231` bytes (~1.77 GiB)
- **Format:** Windows PE32 x86 SFX (Advanced Installer `ADVINSTSFX` containing embedded MSI/CAB archives)
- **Documentation:** `local-input/Readme.pdf` (SHA-256: `5e785074368c61c450de63e81daff310cc1be024859cb78e5a97d8ea8fb5bd39`)
- **Edition:** Need for Speed Underground 2 PC (Win EN v1.2 release)

---

## 2. Directory Structure After Extraction

```text
open-nfs/
├── local-input/                     # (Gitignored) Reference installers & packages
│   ├── README.md                    # Placement guidelines
│   ├── Readme.pdf                   # Installer documentation
│   └── SPEED2.exe                   # 1.85 GB SFX archive
├── game-assets/                     # (Gitignored) Extracted game filesystem (1,579 files)
│   ├── CARS/                        # Vehicle geometry, textures, vinyls, audio configs
│   ├── TRACKS/                      # Track stream bundles (.BUN), world geometry
│   ├── GLOBAL/                      # Global textures, palettes, GlobalB.lzc
│   ├── SOUND/                       # Audio banks (.abk), sound effects
│   ├── SPEECH/                      # Narrative and police speech archives (.big, .idx)
│   ├── MOVIES/                      # EA VP6 cutscenes (.vp6)
│   ├── FRONTEND/                    # UI screens, textures, hud assets
│   ├── LANGUAGES/                   # Multi-language string tables (.bin)
│   └── SPEED2.EXE                   # Original PC executable (v1.2, 4.78 MB)
└── reverse-engineering/
    ├── binaries/                    # (Gitignored) Reference binaries for RE
    │   ├── SPEED2.EXE               # Target binary analyzed via Ghidra / REA
    │   └── repack-extras/           # Isolated mods, ASI loader (dinput8.dll), tools
    └── extraction/                  # Extraction documentation and tooling
        └── README.md                # This document
```

---

## 3. Reference Binary vs. Third-Party Wrappers

To ensure clean reverse engineering of original EA code and avoid analyzing third-party injected hooks:
- **`reverse-engineering/binaries/SPEED2.EXE`**: Contains the pure game executable (SHA-256: `82881d607ea8d90a2898ba9a6ed06da1ff6b56243f2e9146df30226fff69574b`).
- **`reverse-engineering/binaries/repack-extras/`**: Isolates ThirteenAG's Widescreen Fix (`dinput8.dll`, `scripts/NFSUnderground2.WidescreenFix.asi`), Universal Widescreen (`uniws.exe`), and helper wrappers.

---

## 4. Verification & Legal Compliance (ADR-008, NFR-009)

1. **Strict Exclusion**: All files under `local-input/`, `game-assets/`, and `reverse-engineering/binaries/` are excluded by `.gitignore`.
2. **Metadata Only**: Only reproducible hashes, offsets, and technical specifications are committed to `docs/findings/` and `docs/formats/`.
3. **Audit Trail**: Every file has its SHA-256 digest recorded in `docs/findings/M0-manifest.md`.
