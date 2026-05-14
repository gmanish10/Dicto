import { useEffect, useState } from "react";
import {
  ApiKeyStatus,
  MicrophoneInfo,
  PermissionsSnapshot,
  PolishProvider,
  api,
} from "../lib/ipc";
import { HotkeyBinder } from "../components/HotkeyBinder";
import { ApiKeyInput } from "../components/ApiKeyInput";
import { PermissionRow } from "../components/PermissionRow";
import { PolishProviderHelp } from "../components/PolishProviderHelp";
import { POLISH_META, VISIBLE_PROVIDERS } from "../lib/polishLabels";
import { useSettings } from "../hooks/useSettings";
import { emit } from "@tauri-apps/api/event";

const SECTIONS = [
  { id: "general", label: "General" },
  { id: "transcription", label: "Speech-to-text" },
  { id: "cleanup", label: "Cleanup" },
  { id: "privacy", label: "Privacy" },
  { id: "keys", label: "API keys" },
  { id: "permissions", label: "Permissions" },
] as const;

export default function Settings() {
  const { settings, update } = useSettings();
  const [mics, setMics] = useState<MicrophoneInfo[]>([]);
  const [keys, setKeys] = useState<ApiKeyStatus[]>([]);
  const [perms, setPerms] = useState<PermissionsSnapshot | null>(null);
  const [clearingHistory, setClearingHistory] = useState(false);

  const reloadKeys = async () => setKeys(await api.getApiKeyStatus());

  useEffect(() => {
    void api.listMicrophones().then(setMics).catch(() => undefined);
    void reloadKeys();
    void api.checkPermissions().then(setPerms);
  }, []);

  if (!settings) return null;

  return (
    <div className="flex gap-8">
      <SectionNav />
      <div className="min-w-0 flex-1 space-y-10 pb-16">
        {/* --- General --- */}
        <section id="general">
          <SectionHeader title="General" subtitle="Hotkey, microphone, sound effects." />
          <div className="space-y-3">
            <div className="card">
              <span className="label">Hotkey</span>
              <p className="mb-2 text-xs text-ink-400">
                Hold to start recording, release to transcribe and paste.
              </p>
              <HotkeyBinder
                value={settings.hotkey.chord}
                onChange={async (chord) => {
                  await api.setHotkey(chord);
                  await update({ hotkey: { chord } });
                  await emit("settings:updated");
                }}
              />
            </div>

            <div className="card">
              <label className="label" htmlFor="microphone-select">
                Microphone
              </label>
              <select
                id="microphone-select"
                className="input"
                value={settings.microphone_name ?? ""}
                onChange={(e) =>
                  update({
                    microphone_name: e.target.value === "" ? null : e.target.value,
                  })
                }
              >
                <option value="">System default</option>
                {mics.map((m) => (
                  <option key={m.name} value={m.name}>
                    {m.name}
                    {m.is_default ? " (default)" : ""}
                  </option>
                ))}
              </select>
            </div>

            <div className="card grid grid-cols-2 gap-3">
              <label className="flex items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  checked={settings.play_start_chime}
                  onChange={(e) => update({ play_start_chime: e.target.checked })}
                />
                Play start chime
              </label>
              <label className="flex items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  checked={settings.play_stop_chime}
                  onChange={(e) => update({ play_stop_chime: e.target.checked })}
                />
                Play stop chime
              </label>
            </div>

            <div className="card">
              <label className="flex items-start gap-2 text-sm">
                <input
                  type="checkbox"
                  className="mt-0.5"
                  checked={settings.show_recording_overlay}
                  onChange={(e) =>
                    update({ show_recording_overlay: e.target.checked })
                  }
                />
                <span>
                  Show recording indicator
                  <span className="ml-2 text-xs text-ink-400">
                    A small pill at the top of the screen while you're holding the hotkey, so you always know Dicto is listening. Note: macOS hides floating windows over native-fullscreen apps; the start/stop chime still plays.
                  </span>
                </span>
              </label>
            </div>

            <div className="card flex items-center justify-between">
              <div>
                <h3 className="font-medium">Run onboarding again</h3>
                <p className="mt-1 text-xs text-ink-400">
                  Re-walk the Welcome → Permissions → Setup → Try-it flow. Useful after a major
                  release, or when setting Dicto up on a new Mac with the same `~/Library` config.
                </p>
              </div>
              <button
                type="button"
                className="btn-secondary text-xs"
                onClick={async () => {
                  await update({ onboarding_completed: false });
                  window.location.hash = "#/onboarding";
                }}
              >
                Restart onboarding
              </button>
            </div>
          </div>
        </section>

        {/* --- Speech-to-text --- */}
        <section id="transcription">
          <SectionHeader
            title="Speech-to-text"
            subtitle="How your voice is turned into words."
          />
          <div className="card">
            <label className="label" htmlFor="stt-provider">
              Provider
            </label>
            <p className="mb-2 text-xs text-ink-400">
              "Local Whisper" runs entirely on your Mac. Cloud options need your own API key.
            </p>
            <select
              id="stt-provider"
              className="input"
              value={settings.stt_provider}
              onChange={(e) =>
                update({
                  stt_provider: e.target.value as typeof settings.stt_provider,
                })
              }
            >
              <option value="local">Local Whisper — free, on your Mac</option>
              <option value="groq">Groq Whisper — fast cloud, needs Groq key</option>
              <option value="open_ai">OpenAI Whisper — cloud, needs OpenAI key</option>
            </select>
          </div>
        </section>

        {/* --- Cleanup (Polish) --- */}
        <section id="cleanup">
          <SectionHeader
            title="Cleanup"
            subtitle="How Dicto polishes the raw transcript before pasting."
          />
          <div className="card">
            <label className="label" htmlFor="polish-provider">
              Provider
            </label>
            <p className="mb-2 text-xs text-ink-400">
              "Best available" picks the best free option for your Mac automatically.
            </p>
            <select
              id="polish-provider"
              className="input"
              value={settings.polish_provider}
              onChange={(e) =>
                update({ polish_provider: e.target.value as PolishProvider })
              }
            >
              {VISIBLE_PROVIDERS.map((p) => {
                const meta = POLISH_META[p];
                const sub = meta.sublabel ? ` — ${meta.sublabel}` : "";
                return (
                  <option key={p} value={p}>
                    {meta.label}
                    {sub}
                  </option>
                );
              })}
            </select>
            <PolishProviderHelp provider={settings.polish_provider} keys={keys} />
          </div>
        </section>

        {/* --- Privacy --- */}
        <section id="privacy">
          <SectionHeader
            title="Privacy"
            subtitle="What Dicto stores and what leaves your Mac."
          />
          <div className="card space-y-4">
            <div className="text-sm text-ink-700 dark:text-ink-200">
              <p>
                Dicto stores transcripts in a local file at <br />
                <code className="text-xs">~/Library/Application Support/com.dicto.app/dicto.db</code>.
                Nothing is sent anywhere unless you've configured a cloud provider above.
              </p>
            </div>
            <div className="flex flex-wrap gap-2">
              <button
                type="button"
                className="btn-secondary text-xs"
                disabled={clearingHistory}
                onClick={async () => {
                  if (!confirm("Delete all transcript history? This can't be undone.")) return;
                  setClearingHistory(true);
                  try {
                    await api.clearHistory();
                  } finally {
                    setClearingHistory(false);
                  }
                }}
              >
                Clear transcript history
              </button>
            </div>
          </div>
        </section>

        {/* --- API keys (collapsed) --- */}
        <section id="keys">
          <SectionHeader
            title="API keys"
            subtitle="Optional — only needed if you've selected a cloud provider."
          />
          <details className="card">
            <summary className="cursor-pointer text-sm font-medium">
              Show API key inputs
            </summary>
            <p className="mt-3 mb-3 text-xs text-ink-400">
              Keys are stored in the macOS Keychain. Dicto never logs or transmits them
              except to the provider you configured.
            </p>
            <div className="space-y-3">
              <ApiKeyInput
                label="Anthropic"
                provider="anthropic"
                description="Used for Claude Haiku cleanup."
                configured={keys.find((k) => k.key === "anthropic")?.configured ?? false}
                onChanged={reloadKeys}
              />
              <ApiKeyInput
                label="Groq"
                provider="groq"
                description="Used for Groq Whisper transcription and Groq Llama cleanup."
                configured={keys.find((k) => k.key === "groq")?.configured ?? false}
                onChanged={reloadKeys}
              />
              <ApiKeyInput
                label="OpenAI"
                provider="openai"
                description="Used for OpenAI Whisper transcription."
                configured={keys.find((k) => k.key === "openai")?.configured ?? false}
                onChanged={reloadKeys}
              />
            </div>
          </details>
        </section>

        {/* --- Permissions --- */}
        {perms && (
          <section id="permissions">
            <SectionHeader
              title="macOS permissions"
              subtitle="Dicto needs all three to work."
            />
            <div className="space-y-3">
              <PermissionRow
                label="Microphone"
                description="Required to capture your voice."
                status={perms.microphone}
                pane="microphone"
              />
              <PermissionRow
                label="Input Monitoring"
                description="Required for the global hotkey to fire system-wide."
                status={perms.input_monitoring}
                pane="input_monitoring"
              />
              <PermissionRow
                label="Accessibility"
                description="Required to paste cleaned text into other apps."
                status={perms.accessibility}
                pane="accessibility"
              />
            </div>
          </section>
        )}
      </div>
    </div>
  );
}

function SectionHeader({ title, subtitle }: { title: string; subtitle: string }) {
  return (
    <header className="mb-3">
      <h2 className="text-lg font-semibold">{title}</h2>
      <p className="text-sm text-ink-500 dark:text-ink-300">{subtitle}</p>
    </header>
  );
}

function SectionNav() {
  return (
    <nav className="sticky top-2 hidden h-fit w-44 shrink-0 md:block">
      <p className="px-3 pb-2 text-xs uppercase tracking-wide text-ink-400">
        Jump to
      </p>
      <ul className="space-y-0.5 text-sm">
        {SECTIONS.map((s) => (
          <li key={s.id}>
            <a
              href={`#${s.id}`}
              className="block rounded-md px-3 py-1.5 text-ink-700 hover:bg-ink-100 dark:text-ink-200 dark:hover:bg-ink-700"
            >
              {s.label}
            </a>
          </li>
        ))}
      </ul>
    </nav>
  );
}
