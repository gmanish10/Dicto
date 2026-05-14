import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  ApiKeyStatus,
  BundledLlmStatus,
  DownloadProgress,
  PolishProvider,
  api,
} from "../lib/ipc";
import { POLISH_META } from "../lib/polishLabels";

interface Props {
  provider: PolishProvider;
  keys: ApiKeyStatus[];
}

/**
 * Plain-language help panel rendered below the polish-provider dropdown.
 *
 * Always shows the same four fields (What it does, Privacy, Speed, Cost)
 * so users can compare options apples-to-apples. For `bundled_llm`, also
 * surfaces a Download button + live progress bar when the model isn't
 * yet on disk.
 */
export function PolishProviderHelp({ provider, keys }: Props) {
  const meta = POLISH_META[provider];
  const bundled = useBundledLlmStatus();
  const status = computeStatus(provider, keys, bundled.status, bundled.startDownload);

  return (
    <div className="card mt-2 space-y-3">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="font-medium">{meta.label}</div>
          {meta.sublabel && <div className="text-xs text-ink-400">{meta.sublabel}</div>}
        </div>
        <StatusPill kind={status.kind} text={status.text} />
      </div>

      <Field label="What it does">{meta.description}</Field>
      <Field label="Privacy">{meta.privacy}</Field>
      <Field label="Speed">{meta.speed}</Field>
      {meta.cost && <Field label="Cost">{meta.cost}</Field>}

      {status.action && (
        <div className="pt-1">
          <button
            type="button"
            className="btn-secondary text-xs"
            onClick={status.action.onClick}
            disabled={status.action.disabled}
          >
            {status.action.label}
          </button>
        </div>
      )}

      {provider === "bundled_llm" && bundled.status?.downloading && (
        <DownloadProgressBar progress={bundled.status.downloading} />
      )}
    </div>
  );
}

function DownloadProgressBar({ progress }: { progress: DownloadProgress }) {
  const pct =
    progress.total > 0 ? Math.min(100, Math.round((progress.bytes / progress.total) * 100)) : 0;
  const mb = (n: number) => (n / 1024 / 1024).toFixed(0);
  return (
    <div className="space-y-1">
      <div className="h-2 overflow-hidden rounded bg-ink-100 dark:bg-ink-700">
        <div
          className="h-full bg-accent transition-[width]"
          style={{ width: `${pct}%` }}
        />
      </div>
      <div className="text-xs text-ink-400">
        Downloading: {mb(progress.bytes)} MB
        {progress.total > 0 ? ` / ${mb(progress.total)} MB (${pct}%)` : "…"}
      </div>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="grid grid-cols-[5.5rem_1fr] gap-3 text-sm">
      <div className="text-ink-400">{label}</div>
      <div className="text-ink-700 dark:text-ink-200">{children}</div>
    </div>
  );
}

type StatusKind = "ready" | "needs-setup" | "downloading" | "unavailable";

interface ComputedStatus {
  kind: StatusKind;
  text: string;
  action?: { label: string; onClick: () => void; disabled?: boolean };
}

function StatusPill({ kind, text }: { kind: StatusKind; text: string }) {
  const className =
    kind === "ready"
      ? "pill-green"
      : kind === "needs-setup"
      ? "pill-yellow"
      : kind === "downloading"
      ? "pill bg-blue-100 text-blue-700 dark:bg-blue-900/40 dark:text-blue-200"
      : "pill bg-ink-200 text-ink-600 dark:bg-ink-700 dark:text-ink-300";
  return <span className={className}>{text}</span>;
}

function computeStatus(
  provider: PolishProvider,
  keys: ApiKeyStatus[],
  bundled: BundledLlmStatus | null,
  startDownload: () => void
): ComputedStatus {
  const has = (k: "anthropic" | "groq" | "openai") =>
    keys.find((kk) => kk.key === k)?.configured ?? false;

  switch (provider) {
    case "auto":
    case "local_lite":
    case "none":
      return { kind: "ready", text: "Ready" };

    case "apple_intelligence":
      return { kind: "unavailable", text: "Coming soon" };

    case "bundled_llm": {
      if (!bundled) {
        return { kind: "unavailable", text: "Checking…" };
      }
      if (bundled.downloading) {
        return { kind: "downloading", text: "Downloading…" };
      }
      if (bundled.downloaded) {
        return { kind: "ready", text: "Ready" };
      }
      return {
        kind: "needs-setup",
        text: "Needs download",
        action: {
          label: `Download ${bundled.size_mb} MB model`,
          onClick: startDownload,
        },
      };
    }

    case "claude":
      return has("anthropic")
        ? { kind: "ready", text: "Ready" }
        : {
            kind: "needs-setup",
            text: "Needs API key",
            action: {
              label: "Get an Anthropic key",
              onClick: () => window.open("https://console.anthropic.com/", "_blank"),
            },
          };

    case "groq_llama":
      return has("groq")
        ? { kind: "ready", text: "Ready" }
        : {
            kind: "needs-setup",
            text: "Needs API key",
            action: {
              label: "Get a Groq key",
              onClick: () => window.open("https://console.groq.com/keys", "_blank"),
            },
          };
  }
}

/**
 * Polls polish-availability on mount, listens for download progress
 * events, exposes a `startDownload` action. Self-contained — the help
 * component only needs to read `.status` and call `.startDownload`.
 */
function useBundledLlmStatus() {
  const [status, setStatus] = useState<BundledLlmStatus | null>(null);

  const refresh = async () => {
    try {
      const a = await api.checkPolishAvailability();
      setStatus(a.bundled_llm);
    } catch {
      // backend error — leave status null so UI shows "Checking…"
    }
  };

  useEffect(() => {
    void refresh();

    const progressUnlisten = listen<DownloadProgress>(
      "polish-model:download-progress",
      (e) => {
        setStatus((prev) => ({
          downloaded: prev?.downloaded ?? false,
          size_mb: prev?.size_mb ?? 940,
          downloading: e.payload,
        }));
      }
    );
    const completeUnlisten = listen("polish-model:download-complete", () => {
      void refresh();
    });
    const failedUnlisten = listen<string>("polish-model:download-failed", () => {
      void refresh();
    });

    return () => {
      void progressUnlisten.then((fn) => fn());
      void completeUnlisten.then((fn) => fn());
      void failedUnlisten.then((fn) => fn());
    };
  }, []);

  const startDownload = async () => {
    // Optimistic UI: show 0/0 immediately so the bar appears before the
    // first progress event arrives.
    setStatus((prev) => ({
      downloaded: prev?.downloaded ?? false,
      size_mb: prev?.size_mb ?? 940,
      downloading: { bytes: 0, total: 0 },
    }));
    try {
      await api.startPolishModelDownload();
    } catch {
      // backend will fire download-failed event; refresh picks it up
      void refresh();
    }
  };

  return { status, startDownload };
}
