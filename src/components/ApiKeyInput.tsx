import { useState } from "react";
import { ApiKey, api } from "../lib/ipc";

interface Props {
  label: string;
  provider: ApiKey;
  configured: boolean;
  description: string;
  onChanged: () => void;
}

export function ApiKeyInput({ label, provider, configured, description, onChanged }: Props) {
  const [value, setValue] = useState("");
  const [busy, setBusy] = useState(false);

  async function save() {
    if (!value.trim()) return;
    setBusy(true);
    try {
      await api.setApiKey(provider, value.trim());
      setValue("");
      onChanged();
    } finally {
      setBusy(false);
    }
  }

  async function clear() {
    setBusy(true);
    try {
      await api.deleteApiKey(provider);
      onChanged();
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="card">
      <div className="mb-2 flex items-center justify-between">
        <div>
          <h3 className="font-medium">{label}</h3>
          <p className="text-xs text-ink-400">{description}</p>
        </div>
        {configured ? (
          <span className="pill-green">configured</span>
        ) : (
          <span className="pill-yellow">not set</span>
        )}
      </div>
      <div className="flex gap-2">
        <input
          type="password"
          className="input"
          placeholder={configured ? "Replace with new key…" : "Paste API key"}
          value={value}
          onChange={(e) => setValue(e.target.value)}
          autoComplete="off"
        />
        <button type="button" className="btn-primary" disabled={busy || !value.trim()} onClick={save}>
          Save
        </button>
        {configured && (
          <button type="button" className="btn-secondary" disabled={busy} onClick={clear}>
            Remove
          </button>
        )}
      </div>
    </div>
  );
}
