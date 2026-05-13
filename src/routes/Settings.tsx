import { useEffect, useState } from "react";
import { ApiKeyStatus, MicrophoneInfo, PermissionsSnapshot, api } from "../lib/ipc";
import { HotkeyBinder } from "../components/HotkeyBinder";
import { ApiKeyInput } from "../components/ApiKeyInput";
import { PermissionRow } from "../components/PermissionRow";
import { useSettings } from "../hooks/useSettings";
import { emit } from "@tauri-apps/api/event";

export default function Settings() {
  const { settings, update } = useSettings();
  const [mics, setMics] = useState<MicrophoneInfo[]>([]);
  const [keys, setKeys] = useState<ApiKeyStatus[]>([]);
  const [perms, setPerms] = useState<PermissionsSnapshot | null>(null);

  const reloadKeys = async () => setKeys(await api.getApiKeyStatus());

  useEffect(() => {
    void api.listMicrophones().then(setMics).catch(() => undefined);
    void reloadKeys();
    void api.checkPermissions().then(setPerms);
  }, []);

  if (!settings) return null;

  return (
    <div className="space-y-8">
      <section>
        <h2 className="mb-3 text-lg font-semibold">Hotkey</h2>
        <p className="mb-3 text-sm text-ink-500 dark:text-ink-300">
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
      </section>

      <section>
        <h2 className="mb-3 text-lg font-semibold">Transcription</h2>
        <div className="grid gap-3">
          <label className="card">
            <span className="label">Speech-to-text provider</span>
            <select
              className="input"
              value={settings.stt_provider}
              onChange={(e) =>
                update({ stt_provider: e.target.value as typeof settings.stt_provider })
              }
            >
              <option value="local">Local Whisper (free, offline)</option>
              <option value="groq">Groq Whisper large-v3-turbo (BYOK — fastest cloud)</option>
              <option value="open_ai">OpenAI Whisper API (BYOK — high accuracy)</option>
            </select>
            <p className="mt-2 text-xs text-ink-400">
              BYOK = Bring Your Own Key. Local mode runs entirely on your Mac and is free; the cloud
              options trade privacy for higher accuracy and faster speeds on long utterances.
            </p>
          </label>
          <label className="card">
            <span className="label">Polish (clean up ums, uhs, false starts)</span>
            <select
              className="input"
              value={settings.polish_provider}
              onChange={(e) =>
                update({ polish_provider: e.target.value as typeof settings.polish_provider })
              }
            >
              <option value="none">None — paste raw transcript</option>
              <option value="local_lite">Local lite (free, basic filler strip)</option>
              <option value="claude">Claude Haiku (BYOK — best quality)</option>
              <option value="groq_llama">Groq Llama (BYOK — fastest)</option>
            </select>
          </label>
          <label className="card">
            <span className="label">Microphone</span>
            <select
              className="input"
              value={settings.microphone_name ?? ""}
              onChange={(e) =>
                update({ microphone_name: e.target.value === "" ? null : e.target.value })
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
          </label>
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
        </div>
      </section>

      <section>
        <h2 className="mb-3 text-lg font-semibold">API keys (BYOK)</h2>
        <p className="mb-3 text-sm text-ink-500 dark:text-ink-300">
          Keys are stored in the macOS Keychain. Optional — Dicto runs offline by default.
        </p>
        <div className="space-y-3">
          <ApiKeyInput
            label="Groq"
            provider="groq"
            description="Used for Groq Whisper STT and Groq Llama polishing."
            configured={keys.find((k) => k.key === "groq")?.configured ?? false}
            onChanged={reloadKeys}
          />
          <ApiKeyInput
            label="OpenAI"
            provider="openai"
            description="Used for OpenAI Whisper API."
            configured={keys.find((k) => k.key === "openai")?.configured ?? false}
            onChanged={reloadKeys}
          />
          <ApiKeyInput
            label="Anthropic"
            provider="anthropic"
            description="Used for Claude Haiku polishing."
            configured={keys.find((k) => k.key === "anthropic")?.configured ?? false}
            onChanged={reloadKeys}
          />
        </div>
      </section>

      {perms && (
        <section>
          <h2 className="mb-3 text-lg font-semibold">Permissions</h2>
          <div className="space-y-3">
            <PermissionRow
              label="Microphone"
              description="Required to capture your voice."
              status={perms.microphone}
              pane="microphone"
            />
            <PermissionRow
              label="Input Monitoring"
              description="Required for the global hotkey."
              status={perms.input_monitoring}
              pane="input_monitoring"
            />
            <PermissionRow
              label="Accessibility"
              description="Required to paste text into other apps."
              status={perms.accessibility}
              pane="accessibility"
            />
          </div>
        </section>
      )}
    </div>
  );
}
