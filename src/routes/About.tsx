import { useEffect, useState } from "react";
import { getVersion, getTauriVersion } from "@tauri-apps/api/app";
import { api } from "../lib/ipc";
import { Logo } from "../components/Logo";

export default function About() {
  const [checking, setChecking] = useState(false);
  const [updateMsg, setUpdateMsg] = useState<string | null>(null);
  const [version, setVersion] = useState<string>("");
  const [tauriVersion, setTauriVersion] = useState<string>("");

  useEffect(() => {
    void getVersion().then(setVersion).catch(() => undefined);
    void getTauriVersion().then(setTauriVersion).catch(() => undefined);
  }, []);

  async function checkUpdates() {
    setChecking(true);
    setUpdateMsg(null);
    try {
      const available = await api.recheckForUpdates();
      setUpdateMsg(available ? "Update available — restart to install." : "You're on the latest version.");
    } catch (e) {
      setUpdateMsg(`Check failed: ${e}`);
    } finally {
      setChecking(false);
    }
  }

  return (
    <div className="space-y-6">
      <header className="flex items-center gap-4">
        <Logo size={72} idSuffix="about" />
        <div>
          <h1 className="text-2xl font-semibold">Dicto</h1>
          <p className="text-sm text-ink-500 dark:text-ink-300">
            A free, open-source push-to-talk dictation app for macOS.
          </p>
          {version && (
            <p className="mt-1 text-xs text-ink-400">
              v{version}
              {tauriVersion && ` · built on Tauri ${tauriVersion}`}
            </p>
          )}
        </div>
      </header>

      <section className="card">
        <h2 className="mb-2 font-medium">Updates</h2>
        <p className="mb-3 text-sm text-ink-500 dark:text-ink-300">
          Dicto checks GitHub Releases on launch and every 6 hours. A red dot appears in the menubar
          when a new release is available.
        </p>
        <div className="flex items-center gap-3">
          <button
            type="button"
            className="btn-secondary text-sm"
            disabled={checking}
            onClick={checkUpdates}
          >
            {checking ? "Checking…" : "Check now"}
          </button>
          {updateMsg && <span className="text-xs text-ink-400">{updateMsg}</span>}
        </div>
      </section>

      <section className="card">
        <h2 className="mb-2 font-medium">Privacy</h2>
        <ul className="ml-5 list-disc space-y-1 text-sm text-ink-500 dark:text-ink-300">
          <li>Local Whisper mode: audio never leaves your Mac.</li>
          <li>BYOK cloud modes: audio is sent only to the provider you explicitly configured.</li>
          <li>Transcript history is stored in a local SQLite file in <code>~/Library/Application Support/Dicto</code>.</li>
          <li>API keys are stored in the macOS Keychain — Dicto itself never logs or transmits them.</li>
          <li>No telemetry. No analytics. No cloud accounts.</li>
        </ul>
      </section>

      <section className="card">
        <h2 className="mb-2 font-medium">Open source</h2>
        <p className="text-sm text-ink-500 dark:text-ink-300">
          MIT licensed. Source, issues, and releases on{" "}
          <a
            href="https://github.com/manishgoyal/Dicto"
            target="_blank"
            rel="noreferrer"
            className="text-accent hover:underline"
          >
            GitHub
          </a>
          .
        </p>
      </section>
    </div>
  );
}
