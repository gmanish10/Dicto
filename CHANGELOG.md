# Changelog

All notable changes to Dicto are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), versioning follows
[SemVer](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.4] - 2026-05-15

Patch release: onboarding and permission-flow fixes, plus UI cleanup.

### Install

**👉 [Download Dicto_0.3.4_aarch64.dmg](https://github.com/gmanish10/Dicto/releases/download/v0.3.4/Dicto_0.3.4_aarch64.dmg)**

Apple Silicon (M-series) only. Existing v0.1.2+ users get this via the built-in
updater (About → Install and restart). First-time install: drag to `/Applications`,
then `xattr -d com.apple.quarantine /Applications/Dicto.app` once.

### Fixed
- **Onboarding "Back" works again.** Returning to the Permissions step no
  longer bounces you straight forward — you can review it with all three
  permissions granted.
- **Microphone permission updates live.** Granting Microphone access is now
  reflected immediately, without quitting and reopening the app.
- **Dicto auto-appears in System Settings.** Granting Accessibility or Input
  Monitoring now pre-lists Dicto in the relevant pane — no more adding it
  manually with the "+" button.

### Changed
- The default cleanup is now **Basic cleanup** (on-device, always available).
  macOS 26 users still get Apple Intelligence auto-selected during onboarding.
- The onboarding Permissions "Continue" button is simply labelled "Continue".
- The onboarding result panel's "Try again" control moved into a panel header.

### Removed
- The redundant "Best available" cleanup option.
- The History "Re-paste" button — use "Copy"; it puts the transcript on the
  clipboard reliably, whereas re-paste couldn't target the right app from
  inside Dicto's own window.
- The Settings "Run onboarding again" card.

## [0.3.3] - 2026-05-15

Patch release: removes the recording pill, fixes onboarding / Settings /
hotkey bugs, refreshes Apple Intelligence cleanup, and recolors the app.

### Install

**👉 [Download Dicto_0.3.3_aarch64.dmg](https://github.com/gmanish10/Dicto/releases/download/v0.3.3/Dicto_0.3.3_aarch64.dmg)**

Apple Silicon (M-series) only. Existing v0.1.2+ users get this via the built-in
updater (About → Install and restart). First-time install: drag to `/Applications`,
then `xattr -d com.apple.quarantine /Applications/Dicto.app` once.

### Removed
- **The floating recording pill is gone.** The on-screen "Listening"
  indicator has been removed. The menubar icon still reflects recording
  state and the start/stop chimes still play.

### Changed
- **Apple Intelligence cleanup applies light fluency fixes.** It now
  corrects clear grammar mistakes and smooths awkward or run-on phrasing,
  while still preserving your meaning, tone, and wording.
- **New background color.** The app background is now a light pastel
  green, replacing the warm cream.

### Fixed
- **Onboarding no longer resumes mid-flow on a normal launch.** It now
  resumes onto the Permissions step *only* after the macOS-forced
  quit+relaunch that a permission grant triggers; fresh installs, normal
  quits, and reinstalls all start at Welcome.
- **Clicking a section in the Settings sidebar no longer blanks the app.**
  The in-page jump links now scroll to the section instead of breaking
  the router.
- **Long-pressing an arrow key no longer starts dictation.** Arrow keys,
  F-keys, and Page Up/Down/Home/End carry an OS-level Fn annotation; a
  long press could spuriously satisfy the Fn hotkey. Fixed.

## [0.3.2] - 2026-05-15

Patch release: audio, model-download, and permissions hardening, plus an
auto-paste toggle.

### Install

**👉 [Download Dicto_0.3.2_aarch64.dmg](https://github.com/gmanish10/Dicto/releases/download/v0.3.2/Dicto_0.3.2_aarch64.dmg)**

Apple Silicon (M-series) only. Existing v0.1.2+ users get this via the built-in
updater (About → Install and restart). First-time install: drag to `/Applications`,
then `xattr -d com.apple.quarantine /Applications/Dicto.app` once.

### Added
- **Auto-paste toggle** (Settings → General). On by default; turn it off to
  have Dicto only copy the cleaned-up text to the clipboard instead of
  pasting it into the focused app — useful when you want to review the
  result before pasting it yourself.

### Changed
- **Accessibility permission no longer shows a premature "denied" pill.**
  After you click "Allow", Dicto now waits a few seconds before flagging the
  permission as denied, so the normal "open System Settings, flip the
  toggle" flow doesn't flash red mid-grant.

### Removed
- Unused `launch_at_login` and `telemetry_opted_in` settings fields. Dicto
  has never had telemetry or a launch-at-login implementation; these were
  dead config keys. Existing `settings.json` files are unaffected — the
  stale keys are simply ignored.

### Security
- **The bundled whisper model is now SHA-256 verified.** `fetch-model.sh`
  checks the downloaded model against a pinned hash (`scripts/models.sha256`)
  and refuses to bundle a corrupt or tampered artifact.
- **Apple Intelligence cleanup no longer logs transcript text.** Dev logs
  record only timing and character counts, never the dictated content.
- The in-memory recording buffer is now capped (~220 MB ceiling) so a stuck
  hotkey can't grow it without bound.

### Fixed
- Interrupted or hash-mismatched model downloads are now deleted instead of
  being left behind as partial files.

## [0.3.1] - 2026-05-15

Patch release: four fixes flagged after testing v0.3.0, plus a complete visual restyle.

### Install

**👉 [Download Dicto_0.3.1_aarch64.dmg](https://github.com/gmanish10/Dicto/releases/download/v0.3.1/Dicto_0.3.1_aarch64.dmg)**

Apple Silicon (M-series) only. Existing v0.1.2+ users get this via the built-in
updater (About → Install and restart). First-time install: drag to `/Applications`,
then `xattr -d com.apple.quarantine /Applications/Dicto.app` once.

### Fixed
- **Chime fired on arrow / F-keys / Page Up-Down / Home-End** when using the
  Fn hotkey. macOS sets `kCGEventFlagMaskSecondaryFn` in the event flags of
  every "function row" key as a side effect — Dicto was treating that as
  "the Fn hotkey is pressed", so each arrow tap played start + stop chimes
  (no transcription, since the recording was under the minimum-length
  threshold). Now only `flagsChanged` events update modifier state; key
  down / key up events don't touch it.
- **Onboarding Welcome screen mis-stated the default hotkey** as ⌃Space —
  it's actually the Fn / 🌐 globe key. Copy updated to match.

### Changed
- **Permissions step UX.** Granted permissions now show a clean status pill
  and a small "Change in System Settings" link rather than an always-present
  "Request" + "Open System Settings" pair. Not-yet-granted permissions show
  a single prominent "Allow" button.
- **Onboarding resumes after a forced relaunch.** macOS sometimes requires
  the app to quit + reopen when you grant Accessibility or Input Monitoring.
  Dicto now persists the current onboarding step to `settings.json`, so
  relaunching mid-flow drops you back exactly where you were instead of
  restarting from Welcome. Already-granted permissions also auto-advance
  past the Permissions step.
- **Warm-minimal visual restyle.** Dropped the pastel lavender/sky/blush
  palette in favor of a single warm amber accent (`#D4894A`) on a cream +
  warm-charcoal substrate. Status pills shifted from soft pastels to
  warm-minimal hues (olive / brick / taupe / amber). Cards lost the hard
  1-px borders in favor of larger radii and soft shadows. Logo recolored:
  warm-charcoal square with cream sound-wave bars and a single amber
  middle bar.

## [0.3.0] - 2026-05-14

Onboarding redesign + pastel visual refresh.

### Install

**👉 [Download Dicto_0.3.0_aarch64.dmg](https://github.com/gmanish10/Dicto/releases/download/v0.3.0/Dicto_0.3.0_aarch64.dmg)**

Apple Silicon (M-series) only. Existing v0.1.2+ users get this via the built-in
updater (About → Install and restart). First-time install: drag to `/Applications`,
then `xattr -d com.apple.quarantine /Applications/Dicto.app` once.

### Added
- **Six-step onboarding.** Welcome → Permissions → Setup → Try it → Discover →
  Done. Each macOS permission is granted on user-initiated click — no more cold
  TCC prompts before the welcome screen renders. The Setup step picks hotkey,
  microphone, speech-to-text provider, and cleanup provider in one place, with
  inline API-key entry only when you select a BYOK option.
- **Try-it sandbox.** Sample prompts to read aloud; raw + cleaned-up output
  appears in the onboarding window. Auto-paste is suppressed while onboarding
  is in progress so the demo doesn't dump text into whatever app you happen to
  have open.
- **"Restart onboarding" button** in Settings → General. Useful after a major
  release or when re-onboarding on a new Mac with the same `~/Library` config.
- **Pastel palette.** New primary lavender accent (`#B5ACE5`) and a soft
  pastel gradient (`sky → lavender → blush`) on hero surfaces — step titles,
  the progress bar, the welcome logo halo. Status pills use pastel mint /
  rose / amber instead of the punchier traffic-light hues. Easier on
  long-session eyes for an app you keep open all day.
- **Auto-pre-select Apple Intelligence** as cleanup provider on macOS 26+
  during onboarding. One fewer decision for non-expert users.

### Fixed
- **Cold permission popups.** v0.2.x launched `CGEventTap` and `cpal`
  enumeration before any UI rendered, which fired Input Monitoring and
  Microphone TCC prompts cold. The runtime spawn is now gated on
  `onboarding_completed` and started via the new `start_runtime` IPC at the
  end of the Permissions step.

### Internal
- `spawn_coordinator` is now idempotent via an `AtomicBool` flag on
  `AppState` — duplicate IPC during onboarding can't spawn a second hotkey
  tap.
- New `scripts/regen-icons.sh` reproduces the bundled `.icns` + PNG icon set
  from the inline brand SVG using `@resvg/resvg-js` + `tauri icon`.

## [0.2.1] - 2026-05-14

Bug-fix release for four regressions reported on v0.2.0.

### Install

**👉 [Download Dicto_0.2.1_aarch64.dmg](https://github.com/gmanish10/Dicto/releases/download/v0.2.1/Dicto_0.2.1_aarch64.dmg)**

Apple Silicon (M-series) only. Existing v0.1.2+ users get this via the built-in
updater (About → Install and restart). First-time install: drag to `/Applications`,
then `xattr -d com.apple.quarantine /Applications/Dicto.app` once.

### Fixed
- **"Listening" overlay invisible.** The new overlay pill in v0.2.0 was hidden
  behind a solid background — the global Tailwind base layer painted `#root`
  with `bg-ink-50`, covering the transparent Tauri window with a grey/white
  rectangle. The overlay route now also resets `#root` to transparent, so the
  pill paints correctly. (Fullscreen-app limitation from v0.2.0 unchanged.)
- **Start/stop chimes silent.** The Settings toggle and config field existed
  in v0.2.0 but no playback code was actually wired. Dicto now plays Tink on
  recording start and Pop on release via `afplay` with macOS system sounds.
- **Onboarding mic permission detection.** The mic check used cpal's
  `default_input_config()` as a proxy for TCC state, which returns Ok even
  when permission has never been granted. Dicto now calls
  `AVCaptureDevice.authorizationStatusForMediaType:AVMediaTypeAudio` to read
  the actual TCC entry. Onboarding also re-checks permissions on window focus
  so granting in System Settings is picked up the instant you Cmd-Tab back.
- **Polish was rewriting style.** v0.2.0's polish prompt told the model to
  split long sentences into 12-20 word chunks and convert any enumeration
  ("three things…") into a markdown bullet list. Users reported it was
  changing their phrasing and structure. v0.2.1 walks the prompt back to
  pure hygiene: remove fillers, drop stutter repeats, fix capitalization,
  add punctuation. Word choice and sentence structure are preserved exactly,
  and bullets are never auto-inferred.

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
