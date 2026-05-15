# AGENTS.md

## Cursor Cloud specific instructions

### Overview

Dicto is a **macOS-only** Tauri v2 desktop app (Rust backend + React/TypeScript frontend). The Rust backend uses macOS-specific APIs (CoreGraphics, objc2, whisper-rs with CoreML/Metal, macOS Keychain) and **cannot compile or run on Linux**.

### What works on Linux (Cloud Agent environment)

| Check | Command | Notes |
|---|---|---|
| Frontend install | `npm ci` | Uses `package-lock.json` (npm, not pnpm/yarn) |
| TypeScript typecheck | `npm run typecheck` | |
| Frontend build | `npm run build` | `tsc && vite build` |
| Vite dev server | `npm run dev` | Serves React UI on `:1420`; shows "Loading…" without Tauri backend |
| Rust format check | `cargo fmt --all -- --check` | Run from `src-tauri/` |

### What does NOT work on Linux

- `cargo clippy` / `cargo test` / `cargo build` — fails because `whisper-rs` requires macOS CoreML/Metal/Foundation frameworks, and `core-graphics`/`objc2-app-kit` are macOS-only.
- `npx tauri dev` / `npx tauri build` — requires the Rust backend to compile.
- The Swift sidecar (`scripts/build-apple-polish.sh`) — requires Xcode 26 / macOS 26 SDK.

### Node.js version

The project targets **Node.js 20** (per CI). Use `nvm use 20` before running npm commands.

### Rust toolchain

Requires **Rust stable** (latest). The pre-installed toolchain may be outdated; run `rustup update stable` if you see `edition2024` errors. Ensure `rustfmt` and `clippy` components are installed.

### System dependencies for Rust compilation attempts

If attempting partial Rust compilation on Linux, these Tauri system deps are needed:
`libgtk-3-dev libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libasound2-dev libstdc++-14-dev`

### CI reference

CI runs on `macos-14` runners. See `.github/workflows/ci.yml` for the canonical lint/test/build commands:
- Rust: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (all in `src-tauri/`)
- Frontend: `npm ci`, `npm run typecheck`, `npm run build`
