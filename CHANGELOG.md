# Changelog

All notable changes to Dicto are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), versioning follows
[SemVer](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2026-05-14

Patch release. Two bug fixes + an auto-updater that actually works.

### Fixed
- **`[BLANK_AUDIO]` artifact** ([#10](https://github.com/gmanish10/Dicto/issues/10)) — when the
  user held the hotkey without speaking, whisper.cpp's literal `[BLANK_AUDIO]` marker
  was being pasted into the focused app. Now filtered server-side along with
  `[ _BLANK_AUDIO_ ]`, `(silence)`, `[NO_SPEECH]`, and related variants. Silent recordings
  now produce no output, no toast, just a debug log line.
- **Auto-updater manifest pipeline** ([#1](https://github.com/gmanish10/Dicto/issues/1)) —
  release workflow now publishes `Dicto_aarch64.app.tar.gz.sig` and a properly-formatted
  `latest.json` alongside the existing assets, so the in-app updater can find and
  verify update payloads. v0.1.1+ users will get future auto-updates automatically.

### Changed
- Polish stack reorganized into a `Polisher` resolver with auto-fallback (was: hardcoded match).
  Auto becomes the new default for first-time installs; existing settings carry over.
- EnhancedLocalLite cleanup: now strips multi-word soft fillers (`you know`, `i mean`,
  `sort of`, `kind of`, `basically`), expands common contractions (`gonna → going to` etc.),
  capitalizes `i` standalone, detects per-sentence questions, and adds smarter terminal
  punctuation.
- Settings page reorganized into six anchored sections with sticky left-rail nav. API keys
  collapsed by default. New Privacy section with one-click history wipe.
- Polish provider picker now uses plain-language labels with a contextual help panel:
  every option shows *What it does / Privacy / Speed / Cost* + a status pill + recovery
  action buttons (e.g., "Get an Anthropic key" deeplinks to the provider's console).
- Global toast system surfaces backend errors with recovery actions: invalid API key
  → jumps to Settings → keys; mic revoked → opens System Settings; polish provider
  unavailable → opens Cleanup settings.
- README refreshed: build instructions use `npm` instead of `pnpm`; roadmap reorganized
  into v0.1.1 / v0.2.0 / Later with issue links.

### Internal
- Generalized `model::download_file()` + `model::resolve_file()` so future polish-model
  downloads reuse the Whisper-download code path.
- `Intel x86_64` build temporarily removed from the release matrix while
  [#2](https://github.com/gmanish10/Dicto/issues/2) (free-tier `macos-13` runner queue
  policy) is decided. Apple Silicon-only for now.

## [0.1.0] - 2026-05-14

First public build. Push-to-talk dictation with local Whisper transcription.

### Added
- Hold-to-talk hotkey (default `Fn`); custom CGEventTap implementation that
  avoids the rdev / extern-C panic-abort on macOS 26 keycodes.
- On-device transcription via `whisper-rs` + bundled `ggml-small.en` model.
- Free local filler-strip polish (removes "um", "uh", repeated words).
- BYOK cloud options: Groq Whisper, OpenAI Whisper, Anthropic Claude, Groq Llama.
- Custom vocabulary (whisper prompt biasing) + word replacements.
- 20-entry transcript history with style-learning corrections.
- Tray icon + menubar UI; first-run onboarding wizard for the three macOS
  permissions (Microphone, Input Monitoring, Accessibility).
- Auto-updater via GitHub Releases `latest.json`, signature-verified.

### Known limitations
- English-only.
- Unsigned build — users need `xattr -d com.apple.quarantine /Applications/Dicto.app`
  on first install to bypass Gatekeeper.
- Apple Silicon only for v0.1.0 (x86_64 build pending).
