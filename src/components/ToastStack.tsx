import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "../lib/ipc";
import { useNavigate } from "react-router-dom";

type ToastKind = "info" | "warn" | "error";

interface Toast {
  id: number;
  kind: ToastKind;
  message: string;
  action?: { label: string; onClick: () => void };
  /** Hashed by this so we dedupe rapid bursts of the same message. */
  dedupKey: string;
}

const DEFAULT_DURATION_MS: Record<ToastKind, number> = {
  info: 4000,
  warn: 6000,
  error: 8000,
};

/**
 * Global toast surface, mounted once in `App.tsx`.
 *
 * Subscribes to `pipeline:*` events from the Rust backend + polls macOS
 * permissions. Renders a stack at bottom-right. Dedupes the same message
 * fired within 30s so error storms don't spam.
 */
export function ToastStack() {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const nextId = useRef(1);
  const recentKeys = useRef<Map<string, number>>(new Map());
  const navigate = useNavigate();

  const push = useCallback(
    (kind: ToastKind, message: string, action?: Toast["action"], dedupKey?: string) => {
      const key = dedupKey ?? `${kind}:${message}`;
      const now = Date.now();
      const last = recentKeys.current.get(key);
      if (last && now - last < 30_000) return;
      recentKeys.current.set(key, now);

      const id = nextId.current++;
      const toast: Toast = { id, kind, message, action, dedupKey: key };
      setToasts((prev) => [...prev, toast]);

      const duration = DEFAULT_DURATION_MS[kind];
      window.setTimeout(() => {
        setToasts((prev) => prev.filter((t) => t.id !== id));
      }, duration);
    },
    []
  );

  // Subscribe to pipeline events from Rust.
  useEffect(() => {
    const unlisteners: Array<Promise<() => void>> = [];

    unlisteners.push(
      listen<string>("pipeline:error", (e) => {
        const msg = String(e.payload ?? "Something went wrong.");
        // Map common error substrings to friendlier copy + actions.
        if (msg.toLowerCase().includes("api key")) {
          push("error", "Cloud cleanup failed: API key not set or invalid.", {
            label: "Open Settings",
            onClick: () => navigate("/settings#keys"),
          });
          return;
        }
        if (msg.toLowerCase().includes("model")) {
          push("error", "Speech model couldn't be loaded. Try restarting Dicto.");
          return;
        }
        push("error", msg);
      })
    );

    unlisteners.push(
      listen<string>("pipeline:toast", (e) => {
        const msg = String(e.payload ?? "");
        if (!msg) return;
        // Polish-downgrade toasts include the words "wasn't available"; surface
        // a Settings link so users can adjust their preference.
        if (msg.toLowerCase().includes("cleanup wasn't available")) {
          push("info", msg, {
            label: "Open Cleanup settings",
            onClick: () => navigate("/settings#cleanup"),
          });
          return;
        }
        // Secure-input clipboard toast gets a neutral info style.
        push("info", msg);
      })
    );

    unlisteners.push(
      listen<string>("pipeline:warning", (e) => {
        push("warn", String(e.payload ?? "Heads up."));
      })
    );

    return () => {
      unlisteners.forEach((p) => void p.then((fn) => fn()));
    };
  }, [push, navigate]);

  // Poll mic permission every 10s. If it flips from granted to denied, toast.
  useEffect(() => {
    let lastMicStatus: string | null = null;
    const tick = async () => {
      try {
        const snap = await api.checkPermissions();
        if (lastMicStatus === "granted" && snap.microphone !== "granted") {
          push(
            "warn",
            "Microphone access was revoked. Re-enable it to keep using Dicto.",
            {
              label: "Open System Settings",
              onClick: () => api.openSystemSettings("microphone"),
            },
            "perm-mic-revoked"
          );
        }
        lastMicStatus = snap.microphone;
      } catch {
        // Permission probe failed — usually transient, ignore.
      }
    };
    void tick();
    const id = window.setInterval(tick, 10_000);
    return () => window.clearInterval(id);
  }, [push]);

  if (toasts.length === 0) return null;

  return (
    <div className="pointer-events-none fixed bottom-4 right-4 z-50 flex w-80 flex-col gap-2">
      {toasts.map((t) => (
        <div
          key={t.id}
          role="status"
          className={[
            "pointer-events-auto rounded-md border px-3 py-2 text-sm shadow-md",
            t.kind === "error"
              ? "border-red-300 bg-red-50 text-red-900 dark:border-red-700/60 dark:bg-red-900/40 dark:text-red-100"
              : t.kind === "warn"
              ? "border-yellow-300 bg-yellow-50 text-yellow-900 dark:border-yellow-700/60 dark:bg-yellow-900/30 dark:text-yellow-100"
              : "border-ink-200 bg-white text-ink-900 dark:border-ink-700 dark:bg-ink-800 dark:text-ink-100",
          ].join(" ")}
        >
          <div className="flex items-start gap-2">
            <div className="min-w-0 flex-1">{t.message}</div>
            <button
              type="button"
              aria-label="Dismiss"
              className="-mr-1 -mt-1 px-1 text-xs text-ink-400 hover:text-ink-700 dark:hover:text-ink-200"
              onClick={() => setToasts((prev) => prev.filter((x) => x.id !== t.id))}
            >
              ✕
            </button>
          </div>
          {t.action && (
            <div className="mt-2">
              <button
                type="button"
                className="text-xs font-medium text-accent hover:underline"
                onClick={() => {
                  t.action!.onClick();
                  setToasts((prev) => prev.filter((x) => x.id !== t.id));
                }}
              >
                {t.action.label} →
              </button>
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
