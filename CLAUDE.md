# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Dicto is a macOS-only push-to-talk dictation app: a Tauri v2 desktop app with a
Rust backend (`src-tauri/`) and a React + TypeScript frontend (`src/`). Hold a
hotkey → record mic → transcribe → clean up → paste into the focused app.

## Commands

All Rust commands run from `src-tauri/`; all npm commands from the repo root.

```bash
npm install                              # JS deps
./scripts/fetch-model.sh ggml-small.en   # one-time: ~250 MB whisper model into src-tauri/resources/models
npx tauri dev                            # run the app (spawns vite + cargo)
npx tauri build                          # release .dmg → src-tauri/target/release/bundle/dmg/

npm run typecheck                        # tsc --noEmit — frontend gate in CI
npm run build                            # tsc && vite build

# from src-tauri/ — these are the exact CI gates, all must pass:
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings   # warnings are errors
cargo test
cargo test --features no-whisper            # run tests without linking whisper.cpp (faster, no model needed)
cargo test polish::local_lite::tests        # single test module
```

`fetch-model.sh` must run before `tauri build` — the model is bundled as a
resource and the bundler fails without it.

## Releasing

`./scripts/release.sh X.Y.Z` bumps the version in `package.json`,
`src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json` (all three must match),
commits, and tags `vX.Y.Z`. Pushing the tag triggers `.github/workflows/release.yml`,
which builds the `.dmg`, signs the updater artifact, and publishes a draft
GitHub Release plus `latest.json` for the in-app auto-updater. Add the
CHANGELOG.md entry before letting the script commit. Also update Github Issues if when when needed. Any backlog items should be maintained in Github Issues

## Architecture

### The dictation pipeline (`src-tauri/src/pipeline.rs`)

`spawn_coordinator` starts a long-lived coordinator that owns the whole
hotkey → text flow. It is **not** spawned at app launch if onboarding is
incomplete — the `start_runtime` command (chained from `finish_onboarding`)
spawns it later. This is deliberate: the CGEventTap triggers the macOS Input
Monitoring permission prompt, and we don't want that firing before the user
has seen onboarding UI explaining why. `runtime_started` (an `AtomicBool`)
makes the spawn idempotent.

A dedicated `dicto-recorder` thread owns the cpal `Stream` (keeps the rest of
the pipeline `Send` so it can live on tokio). Flow per utterance:
hotkey down → cpal ring buffer → hotkey up → resample to 16k mono →
`Transcriber` → `Polisher` → user replacements → clipboard + simulated Cmd-V.
Recordings under `MIN_RECORDING_MS` (500ms) are dropped as accidental taps.

### Two pluggable provider traits

- **`Transcriber`** (`transcribe/mod.rs`) — speech→text. Impls: `LocalWhisper`
  (whisper.cpp via whisper-rs, CoreML on Apple Silicon), `GroqTranscriber`,
  `OpenAiTranscriber`. Selected by `Settings.stt_provider`.
- **`Polisher`** (`polish/mod.rs`) — cleans up the raw transcript. Impls:
  `LocalLitePolisher` (heuristics — fillers, contractions, punctuation),
  `BundledLlmPolisher` (Qwen 2.5 1.5B via llama.cpp, ~940 MB downloaded on
  first use), `AppleIntelligencePolisher` (macOS 26+ Foundation Models via a
  Swift sidecar), `ClaudePolisher`, `GroqLlamaPolisher`, `NoOpPolisher`.

`polish/resolver.rs` is the key file: `PolishContext` (on `AppState`) caches
expensive client handles (sidecar process, loaded LLM context). `resolve()`
picks the actual `Polisher` per utterance from the user's preference + runtime
availability + keychain state. For `Auto` it resolves
AppleIntelligence → BundledLlm → LocalLite. When an explicitly-chosen provider
is unavailable it silently degrades and the pipeline emits a `pipeline:toast`.
`sanity_check()` rejects bad LLM output (empty, >3× or <25% length, leading
refusal phrases) and falls back.

### Apple Intelligence sidecar

`swift/dicto-apple-polish.swift` compiles to `src-tauri/binaries/dicto-apple-polish-<target>`,
a Tauri `externalBin`. FoundationModels is macOS-26-only, so the binary is
**checked into git** at each release tag (CI runners lack the macOS 26 SDK).
Rebuild it with `scripts/build-apple-polish.sh` on macOS 26 before tagging.

### State, config, storage

`AppState` (`state.rs`) is the `Arc`-shared singleton: `Settings` (RwLock),
`PipelineState` enum (drives the menubar icon), `HistoryStore`, `PolishContext`,
hotkey channel. `config.rs` defines `Settings` / `SttProvider` / `PolishProvider`
(serde `snake_case` — must match the TS union types in `src/lib/ipc.ts`).
Transcript history lives in SQLite at
`~/Library/Application Support/com.dicto.app/dicto.db`. API keys go in the
macOS Keychain (`keychain/mod.rs`), never the config file.

### macOS-native pieces

The hotkey listener (`hotkey/mac_tap.rs`) is a custom CGEventTap supporting
modifier-only chords (Fn, Right Option). Text injection (`inject/`) writes the
clipboard via `arboard` + synthesizes Cmd-V with raw `CGEvent` — enigo is
deliberately avoided because its main-thread-only TSM calls crash on tokio
workers. `overlay.rs` manages the floating "Listening" pill window
(`recording-overlay`, configured in `tauri.conf.json`).

### Frontend ↔ backend

The frontend talks to Rust only through commands registered in
`lib.rs`'s `invoke_handler` — wrapped in `src/lib/ipc.ts` as `api.*`. Rust
pushes state to the UI via Tauri events (`pipeline:toast`, `overlay:set-visible`,
`nav:goto`). Routes (`src/routes/`): Onboarding, Settings, Dictionary, History,
About, RecordingOverlay. Styling is Tailwind.

## Conventions

- Adding a backend command: write it in `commands.rs`, register it in `lib.rs`'s
  `invoke_handler`, expose it in `src/lib/ipc.ts`.
- Any change to `Settings` / `SttProvider` / `PolishProvider` in `config.rs`
  must be mirrored in the TS types in `src/lib/ipc.ts` (serde uses `snake_case`).
- `llama-cpp-2` is pinned to an exact version (pre-1.0 crate) — re-test polish
  on every `Cargo.lock` bump.
- `cargo clippy` runs with `-D warnings` in CI; the build fails on any warning.
