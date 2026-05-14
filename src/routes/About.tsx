import { useEffect, useState } from "react";
import { getVersion, getTauriVersion } from "@tauri-apps/api/app";
import { api } from "../lib/ipc";
import { Logo } from "../components/Logo";

type UpdateState =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "up-to-date" }
  | { kind: "available"; version: string }
  | { kind: "installing"; version: string }
  | { kind: "error"; message: string };

export default function About() {
  const [updateState, setUpdateState] = useState<UpdateState>({ kind: "idle" });
  const [version, setVersion] = useState<string>("");
  const [tauriVersion, setTauriVersion] = useState<string>("");

  useEffect(() => {
    void getVersion().then(setVersion).catch(() => undefined);
    void getTauriVersion().then(setTauriVersion).catch(() => undefined);
  }, []);

  async function checkUpdates() {
    setUpdateState({ kind: "checking" });
    try {
      const newVersion = await api.recheckForUpdates();
      if (newVersion) {
        setUpdateState({ kind: "available", version: newVersion });
      } else {
        setUpdateState({ kind: "up-to-date" });
      }
    } catch (e) {
      setUpdateState({ kind: "error", message: String(e) });
    }
  }

  async function installUpdate() {
    if (updateState.kind !== "available") return;
    const targetVersion = updateState.version;
    setUpdateState({ kind: "installing", version: targetVersion });
    try {
      // On success this call doesn't return — the binary restarts.
      await api.installPendingUpdate();
    } catch (e) {
      setUpdateState({ kind: "error", message: String(e) });
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
          Dicto checks GitHub Releases for new versions. Click below to check now and install in
          one step — your settings, history, and dictionary all carry over.
        </p>
        <div className="flex flex-wrap items-center gap-3">
          <button
            type="button"
            className="btn-secondary text-sm"
            disabled={updateState.kind === "checking" || updateState.kind === "installing"}
            onClick={checkUpdates}
          >
            {updateState.kind === "checking" ? "Checking…" : "Check for updates"}
          </button>

          {updateState.kind === "available" && (
            <>
              <span className="text-xs text-ink-500 dark:text-ink-300">
                Dicto v{updateState.version} is available.
              </span>
              <button
                type="button"
                className="btn-primary text-sm"
                onClick={installUpdate}
              >
                Install and restart
              </button>
            </>
          )}

          {updateState.kind === "installing" && (
            <span className="text-xs text-ink-400">
              Downloading v{updateState.version}… Dicto will restart automatically.
            </span>
          )}

          {updateState.kind === "up-to-date" && (
            <span className="text-xs text-ink-400">You're on the latest version.</span>
          )}

          {updateState.kind === "error" && (
            <span className="text-xs text-red-500">Update failed: {updateState.message}</span>
          )}
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
            href="https://github.com/gmanish10/Dicto"
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
