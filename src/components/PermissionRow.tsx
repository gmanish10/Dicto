import { PermissionStatus, api } from "../lib/ipc";

interface Props {
  label: string;
  description: string;
  status: PermissionStatus;
  pane: "microphone" | "accessibility" | "input_monitoring";
  onRequest?: () => Promise<void> | void;
}

export function PermissionRow({ label, description, status, pane, onRequest }: Props) {
  const isGranted = status === "granted";
  return (
    <div className="card flex items-start justify-between">
      <div className="pr-4">
        <h3 className="font-medium">{label}</h3>
        <p className="mt-1 text-sm text-ink-500 dark:text-ink-300">{description}</p>
      </div>
      <div className="flex flex-col items-end gap-2">
        {isGranted ? (
          <span className="pill-green">granted</span>
        ) : status === "denied" ? (
          <span className="pill-red">denied</span>
        ) : (
          <span className="pill-yellow">not granted</span>
        )}
        {!isGranted && (
          <div className="flex gap-2">
            {onRequest && (
              <button type="button" className="btn-secondary text-xs" onClick={() => onRequest()}>
                Request
              </button>
            )}
            <button
              type="button"
              className="btn-primary text-xs"
              onClick={() => api.openSystemSettings(pane)}
            >
              Open System Settings
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
