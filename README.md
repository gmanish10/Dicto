# Dicto

> Free, open-source push-to-talk dictation for macOS. Hold a hotkey, speak, release — cleaned-up text appears in whatever app you're using.

Dicto is an open-source alternative to apps like Wispr Flow. It runs entirely on-device by default (using whisper.cpp with CoreML acceleration on Apple Silicon), and optionally lets you bring your own API keys (BYOK) for higher-accuracy cloud models when you want them.

- 🎙 **Push-to-talk hotkey** — hold any key chord (including modifier-only chords like Right Option or Fn) to record. Release to transcribe and inject.
- 🧠 **Local Whisper by default** — your audio never leaves your Mac. Optionally BYOK Groq / OpenAI / Anthropic for better accents, faster long-utterance polish, or LLM cleanup.
- ✍️ **Smart cleanup** — Dicto strips "um", "uh", false starts, fixes capitalization, and adds punctuation. **"Best available"** mode picks the right cleanup engine for your Mac automatically; smarter cloud cleanup is available if you bring an Anthropic or Groq API key.
- 📖 **Custom vocabulary** — bias Whisper toward your jargon, product names, names of teammates.
- 🔁 **Replacements** — say "newline" → `\n`, "k8s" → "Kubernetes", whatever you want.
- 📜 **History** — last 20 transcripts, click to copy or re-paste, edit to teach Dicto your style.
- 🛡 **Privacy-first** — no telemetry, no accounts, no cloud sync. SQLite locally; API keys in macOS Keychain.

---

## Installing

> Dicto v1 is **unsigned**. macOS Gatekeeper will block it on first launch.

1. Download the latest `.dmg` from [Releases](https://github.com/gmanish10/Dicto/releases).
2. Open the `.dmg` and drag `Dicto.app` to `/Applications`.
3. Remove the quarantine flag (one-time):

   ```bash
   xattr -d com.apple.quarantine /Applications/Dicto.app
   ```

   *Alternative: right-click `Dicto.app` → **Open** → confirm. (One-time.)*
4. Launch Dicto. Walk through the onboarding wizard — it asks for three macOS permissions:
   - **Microphone** — to hear you
   - **Input Monitoring** — so the hotkey works while other apps are focused
   - **Accessibility** — so Dicto can paste into other apps
5. In Settings, pick a hotkey (default: **Fn key**) and start talking.

   > For the Fn-key default to work, set **System Settings → Keyboard → "Press 🌐 key to:" → Do Nothing** so macOS doesn't intercept Fn for emoji/dictation.

> Want a signed/notarized build? See [signing.md](docs/signing.md) — we don't ship signed binaries yet, but the CI workflow is set up to wire it in once an Apple Developer Program account is available.

---

## How it works

```
Hotkey down  →  cpal captures mic  →  ring buffer
Hotkey up    →  resample to 16k mono  →  Whisper (local, or Groq/OpenAI with your key)
                                        ↓
                              raw transcript ("um, the, the thing is…")
                                        ↓
                              Cleanup (best available — local heuristics, or Claude/Groq with your key)
                                        ↓
                              user replacements ("newline" → \n)
                                        ↓
                              NSPasteboard + simulated Cmd-V into the focused app
                                        ↓
                              save to ~/Library/Application Support/com.dicto.app/dicto.db
```

Tech stack: **Tauri v2** (Rust backend, React + TypeScript frontend), **whisper.cpp** via `whisper-rs` with CoreML acceleration on Apple Silicon, **cpal** for audio capture, a custom **CGEventTap** for global hold-to-talk hotkeys, and raw **CGEvent** + **NSPasteboard** for text injection.

---

## Building from source

Requirements:
- macOS 13 Ventura or newer
- Xcode Command Line Tools (`xcode-select --install`)
- [Rust](https://rustup.rs) (stable)
- [Node.js 20](https://nodejs.org) and npm (ships with Node)
- `cmake` (for building whisper.cpp). On Homebrew: `brew install cmake`.

```bash
git clone https://github.com/gmanish10/Dicto
cd Dicto
npm install
./scripts/fetch-model.sh ggml-small.en   # ~250 MB, one-time
npx tauri dev
```

For a release `.dmg`:

```bash
npx tauri build
# .dmg lands in src-tauri/target/release/bundle/dmg/
```

---

## BYOK — what does that mean?

**BYOK = Bring Your Own Key.** Dicto works fully offline for free. If you have an API key from one of these providers, you can plug it in (Settings → API keys) to upgrade specific parts of the pipeline:

| Provider | What it does for you | Why you'd want it |
|---|---|---|
| **Groq** (Whisper large-v3-turbo) | Cloud speech-to-text, ~10ms per minute of audio | Better accuracy on hard accents and technical terms than local `small.en`; faster than local on long utterances |
| **OpenAI** (Whisper API) | Cloud speech-to-text | Same as above, slightly different accuracy profile |
| **Anthropic** (Claude Haiku) | LLM polishing | Removes "um", "uh", false starts, and rewrites for grammar — far smarter than the local lite cleaner |
| **Groq** (Llama 3.3 70B) | LLM polishing | Sub-100ms polish, free tier covers most personal use |

Keys are stored in the **macOS Keychain**. Dicto never reads them in plaintext logs, never transmits them anywhere except to the provider you configured.

---

## Privacy

- Local Whisper mode: audio **never** leaves your Mac.
- BYOK cloud modes: audio is sent **only** to the provider you explicitly configured.
- Transcript history is a local SQLite file in `~/Library/Application Support/com.dicto.app/dicto.db`. Clear it from the History tab.
- No telemetry, no analytics, no cloud accounts.

---

## Contributing & roadmap

Planned work, open bugs, and feature ideas all live in [GitHub Issues](https://github.com/gmanish10/Dicto/issues), grouped by [milestone](https://github.com/gmanish10/Dicto/milestones). Past releases and their changelogs are at [Releases](https://github.com/gmanish10/Dicto/releases) — see also [CHANGELOG.md](CHANGELOG.md).

Bugs and ideas welcome; open an issue.

---

## License

[MIT](LICENSE).
