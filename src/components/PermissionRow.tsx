import { PermissionStatus, api } from "../lib/ipc";

interface Props {
  label: string;
  description: string;
  status: PermissionStatus;
  pane: "microphone" | "accessibility" | "input_monitoring";
  /**
   * Optional first-time request hook. macOS permission prompts are
   * one-shot, so denied rows always fall back to the System Settings
   * deep-link instead of retrying this hook.
   */
  onRequest?: () => Promise<void> | void;
}

/**
 * Per-permission card.
 *
 * - **Not granted**: prominent "Allow" button. Rows can provide an
 *   inline system-prompt hook for first-time requests.
 * - **Granted**: clean status pill + a small "Change in System Settings"
 *   link so the user can revoke later without leaving Dicto.
 * - **Denied**: same as not-granted but with the muted-red pill, so the
 *   user can re-open System Settings to flip it.
 */
export function PermissionRow({ label, description, status, pane, onRequest }: Props) {
  const isGranted = status === "granted";
  const isDenied = status === "denied";

  return (
    <div className="card flex items-start justify-between gap-4">
      <div className="min-w-0">
        <h3 className="font-medium">{label}</h3>
        <p className="mt-1 text-sm text-ink-500 dark:text-ink-300">{description}</p>
        {isGranted && (
          <button
            type="button"
            className="mt-2 text-xs text-ink-500 underline decoration-dotted underline-offset-2 hover:text-ink-700 dark:hover:text-ink-200"
            onClick={() => api.openSystemSettings(pane)}
          >
            Change in System Settings
          </button>
        )}
      </div>
      <div className="flex flex-shrink-0 flex-col items-end gap-2">
        {isGranted ? (
          <span className="pill-green">granted</span>
        ) : isDenied ? (
          <span className="pill-red">denied</span>
        ) : (
          <span className="pill-yellow">not granted</span>
        )}
        {!isGranted && (
          <button
            type="button"
            className="btn-primary text-xs"
            onClick={async () => {
              if (isDenied) {
                await api.openSystemSettings(pane);
              } else if (onRequest) {
                await onRequest();
              } else {
                await api.openSystemSettings(pane);
              }
            }}
          >
            Allow
          </button>
        )}
      </div>
    </div>
  );
}
