import { useCallback, useEffect, useState } from "react";
import { api, Settings } from "../lib/ipc";
import { emit, listen } from "@tauri-apps/api/event";

export function useSettings() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [loading, setLoading] = useState(true);

  const reload = useCallback(async () => {
    setLoading(true);
    try {
      const s = await api.getSettings();
      setSettings(s);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  useEffect(() => {
    const unlisten = listen("settings:updated", () => void reload());
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [reload]);

  const update = useCallback(async (patch: Partial<Settings>) => {
    if (!settings) return;
    const next = { ...settings, ...patch };
    setSettings(next);
    await api.setSettings(next);
    await emit("settings:updated");
  }, [settings]);

  return { settings, loading, reload, update };
}
