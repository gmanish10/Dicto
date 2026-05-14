# Changelog

All notable changes to Dicto are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), versioning follows
[SemVer](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-05-14

Feature release. On-device LLM cleanup, multi-step onboarding, a floating
"Listening" indicator, system notifications, and a handful of dictation-pipeline
ergonomics + safety fixes.

### Install

**👉 [Download Dicto_0.2.0_aarch64.dmg](https://github.com/gmanish10/Dicto/releases/download/v0.2.0/Dicto_0.2.0_aarch64.dmg)**

Apple Silicon (M-series) only. Existing v0.1.2+ users get this via the built-in
updater (About → Install and restart). First-time install: drag to `/Applications`,
then `xattr -d com.apple.quarantine /Applications/Dicto.app` once.

### Added
- **Apple Intelligence cleanup** ([#5](https://github.com/gmanish10/Dicto/issues/5)) —
  on macOS 26+ Dicto can polish transcripts using Apple's on-device Foundation
  Models LLM. No API key, no network, ~1.5–2 s on M-series. Available as a
  separate option in Settings → Cleanup, and as the preferred pick under "Best
  available".
- **Multi-step onboarding** ([#6](https://github.com/gmanish10/Dicto/issues/6)) —
  Welcome → Permissions → Try it → Done. The "Try it" step listens for the first
  successful hotkey trigger and confirms both recording + transcribing fire before
  letting you continue, so a fresh install can't silently land in a broken state.
- **Floating "Listening" indicator** — a click-through pill at the top center of
  the primary display while the hotkey is held, so it's never ambiguous whether
  Dicto is recording. Settings → "Show recording indicator" to disable. *Known
  limitation:* macOS hides floating windows over native-fullscreen apps; the
  start/stop chime still plays in that case.
- **System notifications when the window is hidden**
  ([#8](https://github.com/gmanish10/Dicto/issues/8)) — banners now appear when the
  cleanup provider falls back or when paste is blocked by macOS secure input,
  so you never miss the toast just because Dicto's window isn't focused.

### Fixed
- **Stuck-hotkey safety net.** Modifier-key state is now polled against the OS
  every 200 ms in addition to the existing event tap. A brief Fn tap followed by
  no further keyboard activity (e.g., while watching a video) used to leave the
  chord "engaged" indefinitely; the next keystroke would then paste minutes of
  unintended audio. Now auto-released within 200 ms; recordings that exceed the
  max duration are discarded outright rather than transcribed and pasted.
- **Paste-target tracking.** Cmd+V now lands in the app that was focused when
  you triggered the hotkey, even if focus drifted during transcribe/polish. If
  the target app has quit by then, paste falls back to the current frontmost app.
- **Whisper sound annotations stripped.** `[exhales]`, `(wind howling)`,
  `[chuckles]`, `(music playing)` and similar captions whisper occasionally
  picks up from its training data are now removed at the transcribe layer
  before they reach any polish step.
- **Trailing space / newline after polish.** Transcripts ending in
  sentence-terminating punctuation get a trailing space; those ending in a
  bullet or numbered list get a trailing newline. Continued dictation flows
  into the next line/sentence without manual cursor work.
- **Settings migration robustness**
  ([#9](https://github.com/gmanish10/Dicto/issues/9)) — settings now load field by
  field. Adding or renaming a settings field no longer wipes the rest of the
  user's preferences.

### Known limitations
- **Bundled on-device LLM cleanup** ([#4](https://github.com/gmanish10/Dicto/issues/4))
  is plumbed end-to-end but disabled in v0.2.0. `llama-cpp-2` 0.1.146 fails to
  parse valid Qwen 2.5 GGUFs on macOS 26 (Tahoe) — known upstream regression.
  Will re-enable once a fix lands. Apple Intelligence covers the same use case
  on macOS 26.
- **Recording indicator in fullscreen apps.** Native-fullscreen apps create their
  own Space and macOS hides floating windows there. The indicator works
  everywhere else; chime stays as the universal feedback.

## [0.1.2] - 2026-05-14

Patch release fixing a UX bug in the auto-updater itself.

### Fixed
- **Auto-updater "Install" actually installs.** Previously the About → "Check for
  updates" button only *checked* for a new version, then showed "Update available
  — restart to install." But restarting did nothing because the update was never
  downloaded. v0.1.2 adds an explicit "Install and restart" button that downloads
  the signed `.app.tar.gz`, verifies the signature against the embedded pubkey,
  swaps the running `.app`, and restarts Dicto. From v0.1.2 onward, auto-updates
  work end-to-end.

### Heads-up for v0.1.0 / v0.1.1 users

Both v0.1.0 and v0.1.1 shipped with the broken About-page updater. They can
*detect* a new release but can't actually install it. You'll need to manually
download `Dicto_0.1.2_aarch64.dmg` from the release page **one last time**.
From v0.1.2 onward, the About → "Install and restart" button does what it says.

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
