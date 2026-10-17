# M0-03, M0-04, M0-05 & M0-06 — Reference Executable Analysis & Provenance

- **Task IDs:** M0-03, M0-04, M0-05, M0-06
- **Milestone:** M0 (Toolchain & Reference Executable Validation)
- **Status:** Done
- **Requirements:** FR-001, FR-002, EV-001, EV-002, EV-003, NFR-009, ADR-001, ADR-008
- **Date:** 2026-10-17
- **Analysis Tools:**
  - Ghidra `12.1.4_PUBLIC` (Headless Analyzer, decompiler module)
  - REA (`rea-agents@6.5.0` MCP bridge)
  - GNU `objdump` (v2.44)
  - OpenJDK `21.0.12.1+1` (64-Bit)

---

## 1. Reference Executable Identification & Integrity (M0-03, M0-04)

### Primary Target Binary
- **Filesystem Path:** `reverse-engineering/binaries/SPEED2.EXE`
- **Source Relpath:** `game-assets/SPEED2.EXE`
- **File Size:** `4,788,224` bytes (~4.57 MiB)
- **SHA-256 Digest:** `82881d607ea8d90a2898ba9a6ed06da1ff6b56243f2e9146df30226fff69574b`
- **Edition / Version:** Need for Speed Underground 2 PC (US/EU v1.2 release)
- **Compilation Timestamp:** `Fri Oct 29 21:23:10 2004` (stored in PE header)
- **Linker Version:** MSVC 7.0 (`0x0700` — Visual Studio .NET 2002/2003)

### Repack Extras & Isolation
Original game assets were sourced from a standard PC release repack (`local-input/SPEED2.exe`, SHA-256: `5e785074368c61c450de63e81daff310cc1be024859cb78e5a97d8ea8fb5bd39`).
Third-party modifications and wrappers were identified and isolated into `reverse-engineering/binaries/repack-extras/`:
- `dinput8.dll` (ThirteenAG Ultimate ASI Loader / Widescreen Fix wrapper)
- `scripts/NFSUnderground2.WidescreenFix.asi`
- `d3dx9_43.dll` (DirectX 9 runtime helper)
- `uniws.exe` (Universal Widescreen patcher)
- `dimap.dll` (EA DirectInput mapping helper)

---

## 2. Binary Architecture & Layout (M0-05)

### REA Evidence
- **File Headers Evidence ID:** `ev_34b45bbe169b8eb9757fb3f095cc8360edfe6e22949f0981263ca168eeba3270`
- **Section Headers Evidence ID:** `ev_64f536f54c4e691bd3fc712d801318de475282a9fed11f3009884b19fa1206a4`

### PE Section Table
| Section | Virtual Address (VMA) | Virtual Size | File Offset | Raw Size | Flags / Purpose |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `.text` | `0x00401000` | `0x00381a31` | `0x00001000` | `0x00382000` | Code (`EXEC_P`, `ALLOC`, `LOAD`, `READONLY`) |
| `.rdata` | `0x00783000` | `0x000641e7` | `0x00383000` | `0x00065000` | Read-only data, IAT, constant strings |
| `.data` | `0x007e8000` | `0x00035000` | `0x003e8000` | `0x00035000` | Read-write global state and variables |
| `.rsrc` | `0x008bc000` | `0x0003854e` | `0x0041d000` | `0x00039000` | Windows PE resources (icons, manifests) |
| (unnamed) | `0x008f5000` | `0x0003ac08` | `0x00456000` | `0x0003b000` | Initialized data / metadata |

- **Image Base:** `0x00400000`
- **Entry Point RVA:** `0x0035bc01` → **Entry Point VA:** `0x0075bc01`
- **Subsystem:** Windows GUI (`0x0002`), Subsystem Version `4.0`

---

## 3. Disassembly & Ghidra Decompilation Evidence (M0-05)

### 3.1. Entry Point (`WinMainCRTStartup` at `0x0075bc01`)

#### Disassembly
```x86asm
0075bc01: push   $0x18
0075bc03: push   $0x7d39a8
0075bc08: call   0x760ce0
0075bc0d: mov    $0x94,%edi            ; sizeof(OSVERSIONINFOA) = 148 bytes
0075bc12: mov    %edi,%eax
0075bc14: call   0x75c400
0075bc19: mov    %esp,-0x18(%ebp)
0075bc1c: mov    %esp,%esi
0075bc1e: mov    %edi,(%esi)
0075bc20: push   %esi
0075bc21: call   *0x783164             ; KERNEL32!GetVersionExA
...
0075bd58: push   %eax                  ; nCmdShow
0075bd59: push   0x8b8448              ; lpCmdLine
0075bd5f: push   0x8b8444              ; hPrevInstance
0075bd65: call   0x580d60              ; <--- Call WinMain
0075bd6a: add    $0xc,%esp
0075bd78: call   0x75d4c3              ; exit(uVar3)
```

#### Ghidra 12.1.4 C Pseudocode
```c
void func_75bc01(uint param_1, int param_2) {
    ...
    GetVersionExA(&puStack_8);
    uRam008bb5f4 = GetCommandLineA();
    uRam008b8418 = func_0x007619a3();
    iVar2 = func_0x0075d38b();
    uRam008b8454 = uRam008b8450;
    
    // Call WinMain at 0x00580d60
    uVar3 = func_0x00580d60(uRam008b8444, uRam008b8448, uRam008b8450);
    
    func_0x0075d4c3(uVar3);
    func_0x0075d4e5();
    return;
}
```

---

### 3.2. WinMain (`0x00580d60`)

#### Disassembly
```x86asm
00580d60: push   $0x1
00580d62: push   $0x7a0df8             ; Pointer to "speed2.exe" in .rdata
00580d67: call   0x5b7ac0              ; Mutex / single-instance check
00580d6c: add    $0x8,%esp
00580d6f: test   %al,%al
00580d71: je     0x580d7a
00580d73: push   $0x0
00580d75: call   0x75d4d4              ; ExitProcess(0) if already running
00580d7a: mov    0x8(%esp),%eax
00580d7e: mov    0x4(%esp),%ecx
00580d82: push   %eax
00580d83: push   %ecx
00580d84: call   0x57ec70              ; Commandline parsing
00580d89: add    $0x8,%esp
00580d8c: call   0x5807e0              ; Subsystem init
00580d91: test   %al,%al
00580d93: je     0x580d9c
00580d95: call   0x4fef40              ; Engine init branch A
00580d9a: jmp    0x580dd0
...
00580dd0: call   0x581120              ; Main Game Loop
00580dd5: call   0x5cf910              ; Teardown
00580dda: push   $0x0
00580ddc: call   0x75d4d4              ; ExitProcess(0)
```

#### String Reference Verification
- VA `0x007a0df8` (file offset `0x003a0df8`): `"speed2.exe\0"`
- Precedes core gameflow strings:
  - `0x007a0e04`: `"LoadingRegion"`
  - `0x007a0e14`: `"BeginGameFlowUnloadTrack"`
  - `0x007a0e2c`: `"BeginGameFlowSwitchTrack"`

---

## 4. Resource Loading Paths Discovered (M1 Preview / FR-003)

Two distinct file-loading subsystems were identified through import cross-references to `CreateFileA` (IAT RVA `0x003831cc`):

1. **Memory-Mapped Asset Loader (`0x0065a92d`)**:
   - Opens file with flags: `GENERIC_READ (0x80000000)`, `FILE_SHARE_READ (1)`, `OPEN_EXISTING (3)`, `FILE_FLAG_RANDOM_ACCESS (0x10000000)`.
   - Calls `CreateFileMappingA` (`0x007831c8`) with `PAGE_READONLY (2)`.
   - Calls `GetFileSize` (`0x007831c4`).
   - Calls `MapViewOfFile` (`0x007831ac`) with `FILE_MAP_READ (4)`.
   - Returns a structured descriptor `[hFile, hMapping, mappedPtr, fileSize]` into `EDI`.

2. **Sequential / Cached Stream File Reader (`0x006f9d70`)**:
   - Handles buffered streaming for track bundles (`STREAML4RD.BUN`), speech (`UG2_Speech.big`), and audio banks.
   - Reads chunks with `KERNEL32!ReadFile`, validates streaming checksums via `func_0x006fb16f`.

---

## 5. Milestone Gate Validation

| Requirement | Acceptance Criterion | Evidence / Verification | Status |
| :--- | :--- | :--- | :--- |
| **FR-001** | Inventory source files and provenance | `docs/findings/M0-manifest.tsv` (1591 files indexed with SHA-256 and byte sizes) | Met |
| **FR-002** | Analyze reference executable with Ghidra/REA | REA evidence IDs `ev_34b45bbe...` and `ev_64f536f5...`, Ghidra 12.1.4 decompilation | Met |
| **EV-001** | Evidence-linked behavior | Exact VAs, offsets, and disassembly for `0x75bc01`, `0x580d60`, `0x65a92d` | Met |
| **EV-002** | Confirmed vs inferred status | Binary hashes, toolchain versions, and addresses documented | Met |
| **EV-003** | No fabricated fields | All addresses and offsets directly decompiled from reference executable | Met |
| **NFR-009** | Commercial binaries excluded from git | `.gitignore` verified, zero proprietary binaries tracked in git repository | Met |

Milestone **M0** exit gate is fully satisfied.
