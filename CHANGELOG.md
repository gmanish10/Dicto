# Changelog

All notable changes to Dicto are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), versioning follows
[SemVer](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
