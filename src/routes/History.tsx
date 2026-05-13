import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { TranscriptRow, api } from "../lib/ipc";
import { formatDuration, formatRelative } from "../lib/format";

export default function History() {
  const [rows, setRows] = useState<TranscriptRow[]>([]);
  const [editing, setEditing] = useState<number | null>(null);
  const [editValue, setEditValue] = useState("");

  const reload = useCallback(async () => {
    setRows(await api.listHistory(20));
  }, []);

  useEffect(() => {
    void reload();
    const u = listen("transcript:new", () => void reload());
    return () => {
      void u.then((fn) => fn());
    };
  }, [reload]);

  async function copy(text: string) {
    await navigator.clipboard.writeText(text);
  }

  async function reinject(id: number) {
    await api.reinjectTranscript(id);
  }

  async function saveEdit(row: TranscriptRow) {
    await api.recordCorrection(row.raw, editValue);
    setEditing(null);
    await reload();
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">Recent transcripts</h2>
        <button
          type="button"
          className="btn-secondary text-xs"
          onClick={async () => {
            if (confirm("Clear all transcript history?")) {
              await api.clearHistory();
              await reload();
            }
          }}
        >
          Clear history
        </button>
      </div>
      <p className="text-sm text-ink-500 dark:text-ink-300">
        Last 20 transcripts. Editing a transcript here teaches Dicto your style — the corrections
        feed into future polishing.
      </p>
      <div className="space-y-3">
        {rows.length === 0 && (
          <p className="text-sm text-ink-400">No transcripts yet. Hold your hotkey and speak.</p>
        )}
        {rows.map((row) => (
          <div key={row.id} className="card">
            <div className="mb-2 flex items-center justify-between text-xs text-ink-400">
              <div>
                {formatRelative(row.ts)} · {formatDuration(row.duration_ms)} ·{" "}
                {row.provider_stt}
                {row.provider_polish ? ` → ${row.provider_polish}` : ""}
              </div>
              <div className="flex gap-2">
                <button type="button" className="hover:underline" onClick={() => copy(row.polished)}>
                  Copy
                </button>
                <button type="button" className="hover:underline" onClick={() => reinject(row.id)}>
                  Re-paste
                </button>
                <button
                  type="button"
                  className="hover:underline"
                  onClick={() => {
                    setEditing(row.id);
                    setEditValue(row.polished);
                  }}
                >
                  Edit
                </button>
              </div>
            </div>
            {editing === row.id ? (
              <div className="space-y-2">
                <textarea
                  className="input min-h-[5rem] font-mono text-sm"
                  value={editValue}
                  onChange={(e) => setEditValue(e.target.value)}
                />
                <div className="flex justify-end gap-2">
                  <button type="button" className="btn-secondary text-xs" onClick={() => setEditing(null)}>
                    Cancel
                  </button>
                  <button type="button" className="btn-primary text-xs" onClick={() => saveEdit(row)}>
                    Save correction
                  </button>
                </div>
              </div>
            ) : (
              <div className="space-y-1 text-sm">
                <div className="font-mono">{row.polished}</div>
                {row.raw !== row.polished && (
                  <details className="text-xs text-ink-400">
                    <summary className="cursor-pointer">Raw transcript</summary>
                    <div className="mt-1 font-mono">{row.raw}</div>
                  </details>
                )}
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
