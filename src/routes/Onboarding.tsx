import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { listen } from "@tauri-apps/api/event";
import { emit } from "@tauri-apps/api/event";
import {
  ApiKey,
  ApiKeyStatus,
  DownloadProgress,
  MicrophoneInfo,
  PermissionsSnapshot,
  PolishProvider,
  Settings,
  SttProvider,
  api,
} from "../lib/ipc";
import { PermissionRow } from "../components/PermissionRow";
import { ApiKeyInput } from "../components/ApiKeyInput";
import { HotkeyBinder } from "../components/HotkeyBinder";
import { Logo } from "../components/Logo";
import { POLISH_META, VISIBLE_PROVIDERS } from "../lib/polishLabels";

/**
 * v0.3.0 onboarding. Six steps, in order:
 *
 *   1. Welcome — pitch + privacy callout
 *   2. Permissions — three TCC grants, user-initiated
 *   3. Models — hotkey + mic + STT + cleanup provider, with inline BYOK
 *      key entry only when the user picks a key-requiring provider
 *   4. Try it — sample prompts + a result panel that shows the
 *      transcribed + cleaned output. Auto-paste is suppressed
 *      server-side while `onboarding_completed` is false
 *      (see pipeline.rs:432).
 *   5. Discover — Dictionary + History education
 *   6. Done — recap + "Open Dicto" CTA, which calls `finishOnboarding`
 *      (which also spawns the dictation runtime if it hadn't been
 *      spawned earlier).
 *
 * The runtime (hotkey tap + recorder) is **not** started at app
 * launch when onboarding is incomplete — that's gated in
 * `src-tauri/src/lib.rs`. We call `api.startRuntime()` ourselves
 * when transitioning out of the Permissions step so the hotkey is
 * live for Try-it. `spawn_coordinator` is idempotent so subsequent
 * calls are harmless.
 */

const STEPS = [
  { id: "welcome", label: "Welcome" },
  { id: "permissions", label: "Permissions" },
  { id: "models", label: "Setup" },
  { id: "try-it", label: "Try it" },
  { id: "discover", label: "Discover" },
  { id: "done", label: "Done" },
] as const;
type StepId = (typeof STEPS)[number]["id"];

const initialPerms: PermissionsSnapshot = {
  microphone: "not_determined",
  accessibility: "not_determined",
  input_monitoring: "not_determined",
};

interface DemoResult {
  raw: string;
  polished: string;
  polishProvider: string | null;
}

/** Format a download as a percentage, or a byte count when the server
 *  didn't send a Content-Length (total === 0). */
function downloadPct(p: DownloadProgress): string {
  if (p.total > 0) {
    return `${Math.floor((p.bytes / p.total) * 100)}%`;
  }
  return `${Math.floor(p.bytes / 1_000_000)} MB`;
}

export default function Onboarding() {
  const navigate = useNavigate();
  const [stepId, setStepIdInternal] = useState<StepId>("welcome");
  const [settings, setSettings] = useState<Settings | null>(null);
  const [perms, setPerms] = useState<PermissionsSnapshot>(initialPerms);
  const [keys, setKeys] = useState<ApiKeyStatus[]>([]);
  const [mics, setMics] = useState<MicrophoneInfo[]>([]);
  const [demo, setDemo] = useState<DemoResult | null>(null);
  const [tryItState, setTryItState] = useState<"idle" | "recording" | "transcribing">("idle");
  const [sawDemo, setSawDemo] = useState(false);
  const [appleAvailable, setAppleAvailable] = useState(false);
  // Passive speech-model state. The model auto-downloads in the background
  // from app launch; onboarding only mirrors its progress — no button.
  const [modelInstalled, setModelInstalled] = useState<boolean | null>(null);
  const [modelDownload, setModelDownload] = useState<DownloadProgress | null>(null);

  // Step navigation is in-memory only. Onboarding does not persist the
  // step on every transition — it only needs to survive one specific
  // event, the macOS-forced quit+relaunch when the user grants
  // Accessibility or Input Monitoring, which `armResume` handles.
  const setStepId = setStepIdInternal;

  // Initial settings + key status load. Resume onto the Permissions
  // step only when the resume marker was armed (see `armResume`) — i.e.
  // the user initiated an Accessibility / Input-Monitoring grant and
  // macOS force-quit Dicto. Every other launch starts at Welcome.
  useEffect(() => {
    void api.getSettings().then((s) => {
      setSettings(s);
      if (s.onboarding_step === "permissions") {
        setStepIdInternal("permissions");
      }
    });
    void api.getApiKeyStatus().then(setKeys);
    void api
      .checkPolishAvailability()
      .then((a) => setAppleAvailable(a.apple_intelligence.available));
  }, []);

  // Subscribe to the background speech-model download for the whole
  // onboarding session: the Setup step shows passive progress and the
  // Try-it step gates local transcription on the model being ready.
  useEffect(() => {
    void api.checkModelAvailability().then((m) => {
      setModelInstalled(m.installed);
      setModelDownload(m.downloading);
    });
    const unsubs: Array<Promise<() => void>> = [];
    unsubs.push(
      listen<DownloadProgress>("model:download-progress", (e) => {
        setModelDownload(e.payload);
        setModelInstalled(false);
      })
    );
    unsubs.push(
      listen("model:download-complete", () => {
        setModelDownload(null);
        setModelInstalled(true);
      })
    );
    unsubs.push(
      listen("model:download-failed", () => {
        setModelDownload(null);
      })
    );
    return () => {
      unsubs.forEach((p) => void p.then((fn) => fn()));
    };
  }, []);

  // Auto-pre-select Apple Intelligence cleanup on macOS 26+ when the
  // user is still on the default (`local_lite`, or legacy `auto`).
  // Picks the best free option without forcing the user to dig through
  // the dropdown. The ref guard makes this run exactly once — without
  // it, a later explicit pick of "Basic cleanup" would be bounced
  // straight back to Apple Intelligence.
  const didPreselectPolish = useRef(false);
  useEffect(() => {
    if (didPreselectPolish.current || !settings || !appleAvailable) return;
    didPreselectPolish.current = true;
    if (
      settings.polish_provider === "local_lite" ||
      settings.polish_provider === "auto"
    ) {
      void writeSettings({ polish_provider: "apple_intelligence" });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [appleAvailable, settings]);

  const refreshPerms = useCallback(async () => {
    setPerms(await api.checkPermissions());
  }, []);

  // Poll while on the Permissions step. Also re-check on window focus
  // so granting in System Settings and Cmd-Tabbing back picks up the
  // change instantly rather than on the next 1.5 s tick.
  useEffect(() => {
    if (stepId !== "permissions") return;
    void refreshPerms();
    const id = setInterval(refreshPerms, 1500);
    const onFocus = () => void refreshPerms();
    window.addEventListener("focus", onFocus);
    return () => {
      clearInterval(id);
      window.removeEventListener("focus", onFocus);
    };
  }, [stepId, refreshPerms]);

  // Try-it step: subscribe to pipeline state + transcript:new events.
  useEffect(() => {
    if (stepId !== "try-it") return;
    const unsubs: Array<Promise<() => void>> = [];
    unsubs.push(
      listen<number>("pipeline:state", (e) => {
        if (e.payload === 1) setTryItState("recording");
        else if (e.payload === 2) setTryItState("transcribing");
        else setTryItState("idle");
      })
    );
    unsubs.push(
      listen<{
        raw: string;
        polished: string;
        polish_provider: string | null;
      }>("transcript:new", (e) => {
        setDemo({
          raw: e.payload.raw,
          polished: e.payload.polished,
          polishProvider: e.payload.polish_provider,
        });
        setSawDemo(true);
        setTryItState("idle");
      })
    );
    return () => {
      unsubs.forEach((p) => void p.then((fn) => fn()));
    };
  }, [stepId]);

  // Load mics lazily once we reach the Models step. Triggers
  // microphone-related device enumeration, but only after the user has
  // already granted Microphone TCC in step 2.
  useEffect(() => {
    if (stepId !== "models" || mics.length > 0) return;
    void api.listMicrophones().then(setMics).catch(() => undefined);
  }, [stepId, mics.length]);

  const allPermsGranted =
    perms.microphone === "granted" &&
    perms.accessibility === "granted" &&
    perms.input_monitoring === "granted";

  const writeSettings = useCallback(async (patch: Partial<Settings>) => {
    setSettings((cur) => (cur ? { ...cur, ...patch } : cur));
    const cur = await api.getSettings();
    await api.setSettings({ ...cur, ...patch });
    await emit("settings:updated");
  }, []);

  // Arm / disarm the onboarding resume marker. macOS force-quits Dicto
  // when the user grants Accessibility or Input Monitoring; arming the
  // marker right before that deep-link is the only signal that the
  // relaunch should resume onto Permissions rather than restart at
  // Welcome. It's cleared the moment the user leaves the Permissions
  // step (and by `finish_onboarding` server-side).
  const armResume = useCallback(async () => {
    const cur = await api.getSettings();
    await api.setSettings({ ...cur, onboarding_step: "permissions" });
  }, []);
  const clearResume = useCallback(async () => {
    const cur = await api.getSettings();
    if (cur.onboarding_step === "") return;
    await api.setSettings({ ...cur, onboarding_step: "" });
  }, []);

  // Move runtime startup to the moment the user clears Permissions. The
  // hotkey tap + recorder need to be alive for Try-it but spawning at
  // app launch is what caused the cold TCC prompts in v0.2.0.
  // `spawn_coordinator` is idempotent so a duplicate call from React
  // re-mount is harmless.
  const onPermissionsContinue = useCallback(async () => {
    await clearResume();
    await api.startRuntime();
    setStepId("models");
  }, [clearResume]);

  async function finish() {
    await api.finishOnboarding();
    navigate("/settings", { replace: true });
  }

  if (!settings) {
    return <div className="p-8 text-ink-400">Loading…</div>;
  }

  const currentStepIndex = STEPS.findIndex((s) => s.id === stepId);

  return (
    <div className="mx-auto max-w-3xl p-8">
      <div className="mb-6 flex items-center gap-4">
        <div className="relative">
          {stepId === "welcome" && (
            <div
              aria-hidden
              className="absolute inset-0 -m-4 rounded-full bg-gradient-brand opacity-25 blur-xl"
            />
          )}
          <Logo size={56} idSuffix="onboarding" className="relative" />
        </div>
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Welcome to Dicto</h1>
          <p className="text-sm text-ink-400">Hold-to-talk dictation, on your Mac.</p>
        </div>
      </div>

      <Stepper currentIndex={currentStepIndex} />

      <div className="mt-6">
        {stepId === "welcome" && <WelcomeStep onNext={() => setStepId("permissions")} />}
        {stepId === "permissions" && (
          <PermissionsStep
            perms={perms}
            allGranted={allPermsGranted}
            onRequestMic={async () => {
              await api.requestMicrophonePermission();
              await refreshPerms();
            }}
            onRequestInputMonitoring={async () => {
              await armResume();
              await api.requestInputMonitoringPermission();
              await refreshPerms();
            }}
            onOpenInputMonitoringSettings={async () => {
              await armResume();
              await api.openSystemSettings("input_monitoring");
            }}
            onRequestAccessibility={async () => {
              await armResume();
              await api.requestAccessibilityPermission();
              await refreshPerms();
            }}
            onOpenAccessibilitySettings={async () => {
              await armResume();
              await api.openSystemSettings("accessibility");
            }}
            onBack={() => setStepId("welcome")}
            onNext={onPermissionsContinue}
          />
        )}
        {stepId === "models" && (
          <ModelsStep
            settings={settings}
            mics={mics}
            keys={keys}
            appleAvailable={appleAvailable}
            modelInstalled={modelInstalled}
            modelDownload={modelDownload}
            onChange={writeSettings}
            onKeysChanged={async () => setKeys(await api.getApiKeyStatus())}
            onBack={() => setStepId("permissions")}
            onNext={() => setStepId("try-it")}
          />
        )}
        {stepId === "try-it" && (
          <TryItStep
            hotkey={settings.hotkey.chord}
            state={tryItState}
            demo={demo}
            sawDemo={sawDemo}
            sttProvider={settings.stt_provider}
            modelInstalled={modelInstalled}
            modelDownload={modelDownload}
            onRetry={() => setDemo(null)}
            onBack={() => setStepId("models")}
            onNext={() => setStepId("discover")}
          />
        )}
        {stepId === "discover" && (
          <DiscoverStep onBack={() => setStepId("try-it")} onNext={() => setStepId("done")} />
        )}
        {stepId === "done" && (
          <DoneStep
            settings={settings}
            mics={mics}
            onFinish={finish}
            onBack={() => setStepId("discover")}
          />
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Stepper

function Stepper({ currentIndex }: { currentIndex: number }) {
  return (
    <div>
      <ol className="flex items-center gap-2 text-xs">
        {STEPS.map((s, i) => {
          const state = i < currentIndex ? "done" : i === currentIndex ? "active" : "pending";
          const pillClass =
            state === "done"
              ? "bg-accent text-ink-900"
              : state === "active"
              ? "border border-accent text-ink-900 dark:text-ink-100"
              : "border border-ink-200 text-ink-400 dark:border-ink-700";
          return (
            <li key={s.id} className="flex items-center gap-2">
              <span
                className={`flex h-6 w-6 items-center justify-center rounded-full text-[10px] ${pillClass}`}
              >
                {i + 1}
              </span>
              <span
                className={
                  state === "active" ? "font-medium text-ink-900 dark:text-ink-100" : "text-ink-400"
                }
              >
                {s.label}
              </span>
              {i < STEPS.length - 1 && <span className="text-ink-300">→</span>}
            </li>
          );
        })}
      </ol>
      {/* Pastel progress bar — fills as the user advances. */}
      <div className="mt-3 h-1 w-full overflow-hidden rounded-full bg-ink-100 dark:bg-ink-700">
        <div
          className="h-full bg-gradient-brand transition-[width] duration-300"
          style={{ width: `${((currentIndex + 1) / STEPS.length) * 100}%` }}
        />
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Step 1: Welcome

function WelcomeStep({ onNext }: { onNext: () => void }) {
  return (
    <div className="card space-y-4">
      <h2 className="text-xl font-semibold text-gradient-brand">How Dicto works</h2>
      <ul className="ml-4 list-disc space-y-1.5 text-sm text-ink-600 dark:text-ink-300">
        <li>Hold your hotkey. Speak. Release.</li>
        <li>Dicto types the cleaned-up text into whatever app you're using.</li>
        <li>
          The default hotkey is the <kbd className="kbd">Fn</kbd> /{" "}
          <kbd className="kbd">🌐</kbd> globe key (bottom-left of your keyboard). You can pick a
          different one in the next step.
        </li>
        <li>
          <strong>Everything stays on your Mac</strong> unless you opt into a cloud cleanup
          provider with your own API key.
        </li>
      </ul>
      <div className="flex justify-end pt-2">
        <button type="button" className="btn-primary" onClick={onNext}>
          Get started
        </button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Step 2: Permissions

function PermissionsStep({
  perms,
  allGranted,
  onRequestMic,
  onRequestInputMonitoring,
  onOpenInputMonitoringSettings,
  onRequestAccessibility,
  onOpenAccessibilitySettings,
  onBack,
  onNext,
}: {
  perms: PermissionsSnapshot;
  allGranted: boolean;
  onRequestMic: () => Promise<void>;
  onRequestInputMonitoring: () => Promise<void>;
  onOpenInputMonitoringSettings: () => Promise<void>;
  onRequestAccessibility: () => Promise<void>;
  onOpenAccessibilitySettings: () => Promise<void>;
  onBack: () => void;
  onNext: () => void | Promise<void>;
}) {
  return (
    <div className="card space-y-4">
      <div>
        <h2 className="text-xl font-semibold text-gradient-brand">Grant three macOS permissions</h2>
        <p className="mt-1 text-sm text-ink-500 dark:text-ink-300">
          Status pills update automatically as you grant each one — you don't need to come back here.
        </p>
      </div>
      <div className="space-y-3">
        <PermissionRow
          label="Microphone"
          description="So Dicto can capture your voice when you hold the shortcut."
          status={perms.microphone}
          pane="microphone"
          onRequest={onRequestMic}
        />
        <PermissionRow
          label="Input Monitoring"
          description="So the global shortcut works while another app is focused."
          status={perms.input_monitoring}
          pane="input_monitoring"
          onRequest={onRequestInputMonitoring}
          onOpenSettings={onOpenInputMonitoringSettings}
        />
        <PermissionRow
          label="Accessibility"
          description="So Dicto can paste cleaned-up text into whatever app you're typing in."
          status={perms.accessibility}
          pane="accessibility"
          onRequest={onRequestAccessibility}
          onOpenSettings={onOpenAccessibilitySettings}
        />
      </div>
      <div className="flex items-center justify-between pt-2">
        <button type="button" className="btn-secondary" onClick={onBack}>
          Back
        </button>
        <button
          type="button"
          className="btn-primary"
          disabled={!allGranted}
          onClick={() => onNext()}
        >
          Continue
        </button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Step 3: Hotkey + models

function ModelsStep({
  settings,
  mics,
  keys,
  appleAvailable,
  modelInstalled,
  modelDownload,
  onChange,
  onKeysChanged,
  onBack,
  onNext,
}: {
  settings: Settings;
  mics: MicrophoneInfo[];
  keys: ApiKeyStatus[];
  appleAvailable: boolean;
  modelInstalled: boolean | null;
  modelDownload: DownloadProgress | null;
  onChange: (patch: Partial<Settings>) => Promise<void>;
  onKeysChanged: () => Promise<void>;
  onBack: () => void;
  onNext: () => void;
}) {
  const sttKeyMissing =
    (settings.stt_provider === "groq" && !keyConfigured(keys, "groq")) ||
    (settings.stt_provider === "open_ai" && !keyConfigured(keys, "openai"));
  const polishKeyMissing =
    (settings.polish_provider === "claude" && !keyConfigured(keys, "anthropic")) ||
    (settings.polish_provider === "groq_llama" && !keyConfigured(keys, "groq"));
  const canContinue = !sttKeyMissing && !polishKeyMissing;

  return (
    <div className="card space-y-6">
      <div>
        <h2 className="text-xl font-semibold text-gradient-brand">Pick your setup</h2>
        <p className="mt-1 text-sm text-ink-500 dark:text-ink-300">
          You can change any of this later in Settings.
        </p>
      </div>

      <section className="space-y-2">
        <h3 className="text-sm font-semibold uppercase tracking-wide text-ink-500">Hotkey</h3>
        <HotkeyBinder
          value={settings.hotkey.chord}
          onChange={async (chord) => {
            await api.setHotkey(chord);
            await onChange({ hotkey: { chord } });
          }}
        />
      </section>

      <section className="space-y-2">
        <h3 className="text-sm font-semibold uppercase tracking-wide text-ink-500">Microphone</h3>
        <select
          className="input max-w-md"
          value={settings.microphone_name ?? ""}
          onChange={(e) =>
            onChange({ microphone_name: e.target.value === "" ? null : e.target.value })
          }
        >
          <option value="">System default</option>
          {mics.map((m) => (
            <option key={m.name} value={m.name}>
              {m.name}
              {m.is_default ? " (default)" : ""}
            </option>
          ))}
        </select>
      </section>

      <section className="space-y-2">
        <h3 className="text-sm font-semibold uppercase tracking-wide text-ink-500">
          Speech-to-text
        </h3>
        <p className="text-xs text-ink-400">
          Local Whisper runs entirely on your Mac. Cloud options need your own API key.
        </p>
        <select
          className="input max-w-md"
          value={settings.stt_provider}
          onChange={(e) => onChange({ stt_provider: e.target.value as SttProvider })}
        >
          <option value="local">Local Whisper (free, on your Mac)</option>
          <option value="groq">Groq — needs API key</option>
          <option value="open_ai">OpenAI Whisper — needs API key</option>
        </select>
        {settings.stt_provider === "groq" && (
          <ApiKeyInput
            label="Groq API key"
            provider="groq"
            configured={keyConfigured(keys, "groq")}
            description="Used for transcription. Stored in your macOS Keychain."
            onChanged={onKeysChanged}
          />
        )}
        {settings.stt_provider === "open_ai" && (
          <ApiKeyInput
            label="OpenAI API key"
            provider="openai"
            configured={keyConfigured(keys, "openai")}
            description="Used for transcription. Stored in your macOS Keychain."
            onChanged={onKeysChanged}
          />
        )}
        {settings.stt_provider === "local" && (
          <p className="text-xs text-ink-400">
            {modelDownload
              ? `Setting up speech recognition… ${downloadPct(modelDownload)}`
              : modelInstalled
              ? "Speech model ready."
              : modelInstalled === false
              ? "Setting up speech recognition…"
              : "Checking speech model…"}
          </p>
        )}
      </section>

      <section className="space-y-2">
        <h3 className="text-sm font-semibold uppercase tracking-wide text-ink-500">Cleanup</h3>
        <p className="text-xs text-ink-400">
          Polishes the transcript — removes "um"/"uh", adds punctuation, fixes capitalization.
          Word choice and phrasing are preserved.
        </p>
        <select
          className="input max-w-md"
          value={settings.polish_provider}
          onChange={(e) => onChange({ polish_provider: e.target.value as PolishProvider })}
        >
          {VISIBLE_PROVIDERS.map((p) => {
            const meta = POLISH_META[p];
            const disabled = p === "apple_intelligence" && !appleAvailable;
            return (
              <option key={p} value={p} disabled={disabled}>
                {meta.label}
                {meta.sublabel ? ` — ${meta.sublabel}` : ""}
                {disabled ? " (macOS 26+ only)" : ""}
              </option>
            );
          })}
        </select>
        <p className="text-xs text-ink-400">{POLISH_META[settings.polish_provider].description}</p>
        {settings.polish_provider === "claude" && (
          <ApiKeyInput
            label="Anthropic API key"
            provider="anthropic"
            configured={keyConfigured(keys, "anthropic")}
            description="Used for Claude cleanup. Stored in your macOS Keychain."
            onChanged={onKeysChanged}
          />
        )}
        {settings.polish_provider === "groq_llama" && (
          <ApiKeyInput
            label="Groq API key"
            provider="groq"
            configured={keyConfigured(keys, "groq")}
            description="Used for Groq cleanup. Stored in your macOS Keychain."
            onChanged={onKeysChanged}
          />
        )}
      </section>

      <div className="flex items-center justify-between pt-2">
        <button type="button" className="btn-secondary" onClick={onBack}>
          Back
        </button>
        <button type="button" className="btn-primary" disabled={!canContinue} onClick={onNext}>
          {canContinue ? "Continue" : "Add the API key above to continue"}
        </button>
      </div>
    </div>
  );
}

function keyConfigured(keys: ApiKeyStatus[], which: ApiKey): boolean {
  return keys.find((k) => k.key === which)?.configured ?? false;
}

// ---------------------------------------------------------------------------
// Step 4: Try it

const SAMPLE_PROMPTS = [
  "Let's meet at 3 PM, no wait, 4 PM tomorrow to discuss the quarterly numbers.",
  "Email John back about the project — tell him I need the spec by Friday.",
  "Hey, um, just a reminder to pick up groceries on the way home.",
];

function TryItStep({
  hotkey,
  state,
  demo,
  sawDemo,
  sttProvider,
  modelInstalled,
  modelDownload,
  onRetry,
  onBack,
  onNext,
}: {
  hotkey: string;
  state: "idle" | "recording" | "transcribing";
  demo: DemoResult | null;
  sawDemo: boolean;
  sttProvider: SttProvider;
  modelInstalled: boolean | null;
  modelDownload: DownloadProgress | null;
  onRetry: () => void;
  onBack: () => void;
  onNext: () => void;
}) {
  const skipConfirmRef = useRef(false);
  const [confirmingSkip, setConfirmingSkip] = useState(false);

  // Local transcription needs the speech model on disk. While it's still
  // downloading we show a passive "Finishing setup…" line instead of the
  // sample prompts — the hotkey would otherwise fail. Cloud STT providers
  // don't need the model, so they're never gated.
  const modelPending = sttProvider === "local" && modelInstalled !== true;

  function handleContinue() {
    if (sawDemo) {
      onNext();
      return;
    }
    // Soft confirmation if the user is skipping the demo — single
    // extra click, no modal. Closes if they click anywhere else.
    if (skipConfirmRef.current) {
      onNext();
    } else {
      skipConfirmRef.current = true;
      setConfirmingSkip(true);
    }
  }

  const stateLabel =
    state === "recording" ? "Listening…" : state === "transcribing" ? "Cleaning up…" : "Ready";

  return (
    <div className="card space-y-5">
      <div>
        <h2 className="text-xl font-semibold text-gradient-brand">Try it out</h2>
        <p className="mt-1 text-sm text-ink-500 dark:text-ink-300">
          Press and hold <kbd className="kbd">{hotkey}</kbd>, read one of these out loud, and release.
          We'll show you what Dicto heard — nothing gets pasted anywhere during onboarding.
        </p>
      </div>

      {modelPending && (
        <div className="rounded-md border border-ink-200 bg-ink-50 p-4 text-sm text-ink-500 dark:border-ink-700 dark:bg-ink-800">
          Finishing setup…{" "}
          {modelDownload ? `(${downloadPct(modelDownload)})` : ""} Local
          transcription will be ready in a moment.
        </div>
      )}

      <div className="rounded-md border border-ink-200 bg-ink-50 p-4 text-sm dark:border-ink-700 dark:bg-ink-800">
        <div className="mb-2 text-xs font-semibold uppercase tracking-wide text-ink-500">
          Try saying one of these or any other sentence you want to try
        </div>
        <ul className="space-y-1.5 text-ink-700 dark:text-ink-200">
          {SAMPLE_PROMPTS.map((p) => (
            <li key={p}>"{p}"</li>
          ))}
        </ul>
      </div>

      <div className="space-y-3">
        <div className="flex items-center justify-between text-xs">
          <span className="font-semibold uppercase tracking-wide text-ink-500">Status</span>
          <span
            className={
              modelPending
                ? "pill-yellow"
                : state === "recording"
                ? "pill-lavender"
                : state === "transcribing"
                ? "pill-yellow"
                : "pill-green"
            }
          >
            {modelPending ? "Finishing setup…" : stateLabel}
          </span>
        </div>
        {demo ? (
          <ResultPanel demo={demo} onRetry={onRetry} />
        ) : (
          <div className="rounded-md border border-dashed border-ink-300 p-5 text-center text-sm text-ink-400 dark:border-ink-600">
            {modelPending
              ? "One moment — getting speech recognition ready."
              : "Result will appear here after you release the hotkey."}
          </div>
        )}
      </div>

      <div className="flex items-center justify-between pt-2">
        <button type="button" className="btn-secondary" onClick={onBack}>
          Back
        </button>
        <div className="flex items-center gap-3">
          {confirmingSkip && !sawDemo && (
            <span className="text-xs text-ink-500">
              Skip without trying? Click Continue again.
            </span>
          )}
          <button type="button" className="btn-primary" onClick={handleContinue}>
            Continue
          </button>
        </div>
      </div>
    </div>
  );
}

function ResultPanel({ demo, onRetry }: { demo: DemoResult; onRetry: () => void }) {
  return (
    <div className="overflow-hidden rounded-md border border-ink-200 dark:border-ink-700">
      <div className="h-1 bg-gradient-brand" />
      <div className="flex items-center justify-between border-b border-ink-200 px-4 py-2 dark:border-ink-700">
        <span className="text-sm font-medium">Result</span>
        <button type="button" className="text-xs text-ink-500 underline" onClick={onRetry}>
          Try again
        </button>
      </div>
      <div className="space-y-3 p-4">
        <div>
          <div className="mb-1 text-xs font-semibold uppercase tracking-wide text-ink-500">
            Raw transcript
          </div>
          <p className="rounded bg-ink-50 p-3 text-sm dark:bg-ink-800">{demo.raw || "(empty)"}</p>
        </div>
        <div>
          <div className="mb-1 text-xs font-semibold uppercase tracking-wide text-ink-500">
            After cleanup{demo.polishProvider ? ` (${demo.polishProvider})` : ""}
          </div>
          <p className="rounded bg-ink-50 p-3 text-sm dark:bg-ink-800">
            {demo.polished || "(empty)"}
          </p>
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Step 5: Discover

function DiscoverStep({ onBack, onNext }: { onBack: () => void; onNext: () => void }) {
  return (
    <div className="card space-y-5">
      <div>
        <h2 className="text-xl font-semibold text-gradient-brand">Two more things to know</h2>
        <p className="mt-1 text-sm text-ink-500 dark:text-ink-300">
          You can find both of these in the sidebar after you finish onboarding.
        </p>
      </div>

      <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
        <div className="rounded-md border border-ink-200 p-4 dark:border-ink-700">
          <h3 className="mb-1 font-semibold">📖 Dictionary</h3>
          <p className="text-sm text-ink-600 dark:text-ink-300">
            Add proper nouns, jargon, and names that Dicto tends to mishear. Words you add bias the
            transcription model so it gets your custom vocabulary right.
          </p>
        </div>
        <div className="rounded-md border border-ink-200 p-4 dark:border-ink-700">
          <h3 className="mb-1 font-semibold">🕘 History</h3>
          <p className="text-sm text-ink-600 dark:text-ink-300">
            Every transcript stays here, locally. Re-paste a past transcript, copy it, or correct it
            — corrections feed back into future cleanups.
          </p>
        </div>
      </div>

      <div className="flex items-center justify-between pt-2">
        <button type="button" className="btn-secondary" onClick={onBack}>
          Back
        </button>
        <button type="button" className="btn-primary" onClick={onNext}>
          Continue
        </button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Step 6: Done

function DoneStep({
  settings,
  mics,
  onFinish,
  onBack,
}: {
  settings: Settings;
  mics: MicrophoneInfo[];
  onFinish: () => void;
  onBack: () => void;
}) {
  const micLabel = useMemo(() => {
    if (!settings.microphone_name) {
      return mics.find((m) => m.is_default)?.name ?? "System default";
    }
    return settings.microphone_name;
  }, [settings.microphone_name, mics]);

  return (
    <div className="card space-y-4 text-center">
      <div className="text-3xl">✨</div>
      <h2 className="text-xl font-semibold text-gradient-brand">You're all set</h2>
      <p className="mx-auto max-w-md text-sm text-ink-500 dark:text-ink-300">
        Hold your hotkey from anywhere on macOS — Dicto will paste your transcript wherever you're
        typing.
      </p>
      <div className="mx-auto inline-flex flex-wrap items-center justify-center gap-2 text-xs">
        <span className="pill-lavender">Hotkey: {settings.hotkey.chord}</span>
        <span className="pill-lavender">Mic: {micLabel}</span>
        <span className="pill-lavender">
          Cleanup: {POLISH_META[settings.polish_provider].label}
        </span>
      </div>
      <p className="text-xs text-ink-400">Change anything in Settings any time.</p>
      <div className="flex items-center justify-between pt-2">
        <button type="button" className="btn-secondary" onClick={onBack}>
          Back
        </button>
        <button type="button" className="btn-primary" onClick={onFinish}>
          Open Dicto
        </button>
      </div>
    </div>
  );
}
