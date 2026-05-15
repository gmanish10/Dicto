import { useCallback, useEffect, useState } from "react";
import { api, Settings } from "../lib/ipc";
import { emit, listen } from "@tauri-apps/api/event";

export function useSettings() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [loading, setLoading] = useState(true);

  const loadSettings = useCallback(async (showLoading: boolean) => {
    if (showLoading) setLoading(true);
    try {
      const s = await api.getSettings();
      setSettings(s);
    } finally {
      if (showLoading) setLoading(false);
    }
  }, []);

  const reload = useCallback(async () => {
    await loadSettings(true);
  }, [loadSettings]);

  useEffect(() => {
    void reload();
  }, [reload]);

  useEffect(() => {
    const unlisten = listen("settings:updated", () => {
      void loadSettings(false);
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [loadSettings]);

  const update = useCallback(async (patch: Partial<Settings>) => {
    const current = await api.getSettings();
    const next = { ...current, ...patch };
    await api.setSettings(next);
    setSettings(next);
    await emit("settings:updated");
  }, []);

  return { settings, loading, reload, update };
}
