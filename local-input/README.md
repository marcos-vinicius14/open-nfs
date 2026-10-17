# Local Reference Inputs (Gitignored)

Place lawfully obtained PC reference files of Need for Speed Underground 2 in this directory:

- Expected primary executable: `SPEED2.EXE`
- Supporting installation directories/assets (e.g., `CAR_GLOBAL.BUN`, `TRACKS/`, `GLOBAL/`)

### Mandatory Rules (ADR-008, NFR-009, EV-001–005)
1. **Never commit commercial or proprietary files** to version control.
2. The entire contents of this directory are excluded in `.gitignore`.
3. An inventory manifest containing SHA-256 digests and file sizes (without file contents) will be generated in `docs/findings/` for verification.
