# Dicto

> Free, open-source push-to-talk dictation for macOS. Hold a hotkey, speak, release — cleaned-up text appears in whatever app you're using.

Dicto is an open-source alternative to apps like Wispr Flow. It runs entirely on-device by default (using whisper.cpp with CoreML acceleration on Apple Silicon), and optionally lets you bring your own API keys (BYOK) for higher-accuracy cloud models when you want them.

- 🎙 **Push-to-talk hotkey** — hold any key chord (including modifier-only chords like Right Option or Fn) to record. Release to transcribe and inject.
- 🧠 **Local Whisper by default** — your audio never leaves your Mac. Optionally BYOK Groq / OpenAI / Anthropic for better accents, faster long-utterance polish, or LLM cleanup.
- ✍️ **Disfluency cleanup** — "um, like, the the thing is, uh" becomes "We should ship it tomorrow." Built-in local stripper is free; Claude / Groq LLM polish is BYOK.
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
Hotkey up    →  resample to 16k mono  →  Whisper (local or BYOK)
                                        ↓
                              raw transcript ("um, the, the thing is…")
                                        ↓
                              Polisher (local-lite, or Claude/Groq BYOK)
                                        ↓
                              user replacements ("newline" → \n)
                                        ↓
                              NSPasteboard + simulated Cmd-V into focused app
                                        ↓
                              save to ~/Library/Application Support/Dicto/dicto.db
```

Tech stack: **Tauri v2** (Rust backend, React+TS frontend), **whisper.cpp** via `whisper-rs` with CoreML feature on Apple Silicon, **cpal** for audio, a custom **CGEventTap** for global hotkey hold-detection (replaces `rdev`, which panic-aborts in its extern-C callback on macOS 26 keycodes), **arboard** + raw **CGEvent** for clipboard-paste injection (replaces `enigo`, which calls thread-affined TSM APIs from worker threads).

---

## Building from source

Requirements:
- macOS 13 Ventura or newer
- Xcode Command Line Tools (`xcode-select --install`)
- [Rust](https://rustup.rs) (stable)
- [Node.js 20](https://nodejs.org) + [pnpm 9](https://pnpm.io)

```bash
git clone https://github.com/gmanish10/Dicto
cd Dicto
pnpm install
./scripts/fetch-model.sh ggml-small.en   # ~250 MB, one-time
pnpm tauri dev
```

For a release `.dmg`:

```bash
pnpm tauri build
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
- Transcript history is a local SQLite file in `~/Library/Application Support/Dicto/dicto.db`. Clear it from the History tab.
- No telemetry, no analytics, no cloud accounts.

---

## Roadmap

v1 ships English only. Planned:
- Multi-language support (swap in non-`.en` Whisper models)
- Voice Activity Detection (silero-vad) for better silence handling on long utterances
- Code signing + notarization (no more `xattr` workaround)
- Streaming partial transcripts
- Custom polish prompts per app (e.g., terse in Slack, formal in email)

---

## License

[MIT](LICENSE).

Inspired by [Wispr Flow](https://wisprflow.ai/), [Handy](https://github.com/cjpais/Handy), [VoiceInk](https://github.com/Beingpax/VoiceInk), and [OpenQuack](https://github.com/larryxiao/openquack).
