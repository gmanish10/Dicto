import { useCallback, useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { api, PermissionsSnapshot } from "../lib/ipc";
import { PermissionRow } from "../components/PermissionRow";

const initialPerms: PermissionsSnapshot = {
  microphone: "not_determined",
  accessibility: "not_determined",
  input_monitoring: "not_determined",
};

export default function Onboarding() {
  const navigate = useNavigate();
  const [perms, setPerms] = useState<PermissionsSnapshot>(initialPerms);

  const refresh = useCallback(async () => {
    setPerms(await api.checkPermissions());
  }, []);

  // Poll permissions every 1.5s while onboarding is open.
  useEffect(() => {
    void refresh();
    const id = setInterval(refresh, 1500);
    return () => clearInterval(id);
  }, [refresh]);

  const allGranted =
    perms.microphone === "granted" &&
    perms.accessibility === "granted" &&
    perms.input_monitoring === "granted";

  async function complete() {
    await api.finishOnboarding();
    navigate("/settings", { replace: true });
  }

  return (
    <div className="mx-auto max-w-2xl p-8">
      <h1 className="mb-2 text-2xl font-semibold">Welcome to Dicto</h1>
      <p className="mb-6 text-sm text-ink-500 dark:text-ink-300">
        Dicto needs three macOS permissions to work as a global push-to-talk dictation app. Grant
        each in System Settings — the status pills update automatically.
      </p>

      <div className="space-y-4">
        <PermissionRow
          label="Microphone"
          description="So Dicto can capture your voice when you hold the shortcut."
          status={perms.microphone}
          pane="microphone"
          onRequest={async () => {
            await api.requestMicrophonePermission();
            await refresh();
          }}
        />
        <PermissionRow
          label="Input Monitoring"
          description="So the global shortcut works while another app is focused. macOS will prompt the first time you trigger the hotkey."
          status={perms.input_monitoring}
          pane="input_monitoring"
        />
        <PermissionRow
          label="Accessibility"
          description="So Dicto can paste cleaned-up text into whatever app you're typing in."
          status={perms.accessibility}
          pane="accessibility"
        />
      </div>

      <div className="mt-8 flex items-center justify-between">
        <p className="text-sm text-ink-400">
          {allGranted ? "All set — click Continue." : "Continue is enabled once all three are granted."}
        </p>
        <button
          type="button"
          className="btn-primary"
          disabled={!allGranted}
          onClick={complete}
        >
          Continue
        </button>
      </div>
    </div>
  );
}
