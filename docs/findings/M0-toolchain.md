# M0-01 & M0-02 — Toolchain Inventory & REA/Ghidra Integration

- **Task IDs:** M0-01, M0-02
- **Status:** Done
- **Requirements:** FR-001, FR-002, EV-001, EV-002, EV-003, NFR-006, NFR-007
- **Date:** 2026-10-17
- **Environment:** Omarchy Linux (Kernel 7.2.3-arch1-3 x86_64)

---

## 1. Toolchain Inventory (M0-01)

Every tool required for reverse engineering, offline asset transformation, desktop validation, and Android ARM64 compilation has been inventoried with exact versions, architectures, and filesystem locations:

| Tool | Version | Path / Location | Purpose |
| :--- | :--- | :--- | :--- |
| **Rust Compiler** | `rustc 1.94.0 (4a4ef493e 2026-03-02)` | `/home/marcos/.cargo/bin/rustc` | Engine compilation |
| **Cargo** | `cargo 1.94.0 (85eff7c80 2026-01-15)` | `/home/marcos/.cargo/bin/cargo` | Workspace & package management |
| **Rustup Targets** | `x86_64-unknown-linux-gnu`, `aarch64-linux-android` | `/home/marcos/.rustup/toolchains/...` | Desktop testing & Android ARM64 compilation |
| **Cargo NDK** | `cargo-ndk 4.1.2` | `/home/marcos/.cargo/bin/cargo-ndk` | Cargo Android NDK build driver |
| **Android Tools / ADB** | `37.0.0-android-tools (ADB 1.0.41)` | `/usr/bin/adb` | Device inspection, deployment and profiling |
| **JDK (Java)** | OpenJDK `21.0.12.1+1` (64-Bit) | `/usr/lib/jvm/java-21-openjdk` | Runtime requirement for Ghidra 12.1.x |
| **Ghidra** | `12.1.4` (PUBLIC) | `/home/marcos/tools/ghidra_12.1.4_PUBLIC` | Primary reverse engineering and decompiler provider |
| **Headless Analyzer** | Ghidra `analyzeHeadless` | `/home/marcos/tools/ghidra_12.1.4_PUBLIC/support/analyzeHeadless` | Headless batch analysis |
| **Node.js** | `v26.9.0` | `/usr/bin/node` | Runtime for REA MCP server |
| **REA (Reverse Engineer Anything)** | `rea-agents 6.5.0` (Catalog 207 tools) | `/home/marcos/.npm/_npx/.../rea` | Agent reverse engineering MCP bridge |
| **Antigravity (AGY)** | Agentic AI Coding Environment | `/home/marcos/.gemini/antigravity-cli` | Agent runtime |

### Environment Variables Configured
The following variables were persisted to `~/.bashrc` and `~/.profile`:
- `GHIDRA_INSTALL_DIR="/home/marcos/tools/ghidra_12.1.4_PUBLIC"`
- `JAVA_HOME="/usr/lib/jvm/java-21-openjdk"`
- `PATH="/home/marcos/.cargo/bin:/home/marcos/.local/bin:$PATH"`

---

## 2. REA ↔ Ghidra MCP Registration & Configuration (M0-02)

REA is registered as an MCP server for Antigravity in `/home/marcos/.gemini/config/mcp_config.json`:

```json
{
  "mcpServers": {
    "rea": {
      "command": "npx",
      "args": ["-y", "rea-agents@6.5.0", "mcp"],
      "env": {
        "GHIDRA_INSTALL_DIR": "/home/marcos/tools/ghidra_12.1.4_PUBLIC",
        "HOPPER_LAUNCHER_PATH": "/opt/hopper/bin/Hopper",
        "JAVA_HOME": "/usr/lib/jvm/java-21-openjdk"
      }
    }
  }
}
```

Verification with `rea-agents doctor --provider ghidra`:
- `ghidra`: `ok: true` (version 12.1.4)
- `ghidra-platform`: `ok: true` (linux/x64)
- `ghidra-native_decompiler`: `ok: true` (ELF x86_64 decompile binary matches host)
- `ghidra-java`: `ok: true` (OpenJDK 21.0.12.1 meets Java 21+ requirement)
- `registration:antigravity`: `state: aligned` (`package_reference: rea-agents@6.5.0`)

---

## 3. Real Integration Smoke Test Results (M0-02)

To demonstrate that AGY can control Ghidra through REA without human intervention, a live headless analysis smoke test was executed via REA's MCP tool interface:

1. **Session Probe (`rea:binary_session`)**:
   - Successfully inspected server identity, capabilities, and verified Ghidra was marked as `available: true`.

2. **Target Ingestion (`rea:open_binary`)**:
   - Target: `/usr/bin/true`
   - Provider explicitly selected: `ghidra`
   - Target verified: `format: elf`, `kind: executable`, `architecture: x86_64`
   - Digest: `SHA-256: 8bc9a73afd7d85061160857536e093ded799dae7a2f0cf5758c4553f2f83002d`

3. **Analysis Execution (`rea:binary_overview`)**:
   - Ghidra 12.1.4 imported the binary into an ephemeral project, completed auto-analysis, and returned structural breakdown:
     - **Segments detected:** 31 (`.text`, `.rodata`, `.data`, `.dynamic`, `.got`, `.plt`, etc.)
     - **Procedures identified:** 206 functions
     - **Strings extracted:** 154 defined strings
   - Evidence ID: `ev_ef65c7f658ee8bc1abe95169d6594df3b9c5a8fed28dd66c8f41dc4e50f5be35`

4. **Session Teardown (`rea:close_binary`)**:
   - The ephemeral Ghidra analysis session was closed cleanly (`{"result": null}`).

---

## 4. Risks & Next Actions

- **Current Status**: M0-01 and M0-02 are fully validated and documented with reproducible evidence.
- **Pending Dependencies**: M0-03 through M0-06 are deferred until the user's download of the PC game installation files completes.
- **Next Action**: Once the PC game files are placed in `local-input/`, generate the SHA-256 inventory manifest (M0-04) and open `SPEED2.EXE` in REA with Ghidra (M0-05).
