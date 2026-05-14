import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { listen } from "@tauri-apps/api/event";
import { api, PermissionsSnapshot, Settings } from "../lib/ipc";
import { PermissionRow } from "../components/PermissionRow";
import { Logo } from "../components/Logo";

/**
 * Multi-step onboarding: Welcome → Permissions → Try It Out → Done.
 *
 * The "Try It Out" step doubles as a mic-and-hotkey check: when the
 * user holds their hotkey, we see the pipeline transition to Recording
 * (mic captured audio → hotkey wired correctly), then Transcribing
 * (audio reached whisper), then back to Idle. Catching both transitions
 * proves the end-to-end stack works on this machine before they ever
 * leave onboarding — much cheaper than failing silently in real usage.
 */
type StepId = "welcome" | "permissions" | "try-it" | "done";

const STEPS: { id: StepId; label: string }[] = [
  { id: "welcome", label: "Welcome" },
  { id: "permissions", label: "Permissions" },
  { id: "try-it", label: "Try it" },
  { id: "done", label: "Done" },
];

const initialPerms: PermissionsSnapshot = {
  microphone: "not_determined",
  accessibility: "not_determined",
  input_monitoring: "not_determined",
};

export default function Onboarding() {
  const navigate = useNavigate();
  const [stepId, setStepId] = useState<StepId>("welcome");
  const [perms, setPerms] = useState<PermissionsSnapshot>(initialPerms);
  const [hotkeyChord, setHotkeyChord] = useState<string>("");
  const [sawRecording, setSawRecording] = useState(false);
  const [sawTranscribing, setSawTranscribing] = useState(false);

  // Show the user the chord they're supposed to press in step 3.
  useEffect(() => {
    api.getSettings().then((s: Settings) => setHotkeyChord(s.hotkey.chord)).catch(() => {});
  }, []);

  const refreshPerms = useCallback(async () => {
    setPerms(await api.checkPermissions());
  }, []);

  // Poll permissions only while the permissions step is open.
  useEffect(() => {
    if (stepId !== "permissions") return;
    void refreshPerms();
    const id = setInterval(refreshPerms, 1500);
    return () => clearInterval(id);
  }, [stepId, refreshPerms]);

  // While in try-it: subscribe to pipeline:state events and mark the
  // matching observations so the Continue button enables.
  useEffect(() => {
    if (stepId !== "try-it") return;
    const unlisten = listen<number>("pipeline:state", (e) => {
      // 0=Idle, 1=Recording, 2=Transcribing, 3=UpdateAvailable
      if (e.payload === 1) setSawRecording(true);
      if (e.payload === 2) setSawTranscribing(true);
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [stepId]);

  const allPermsGranted =
    perms.microphone === "granted" &&
    perms.accessibility === "granted" &&
    perms.input_monitoring === "granted";

  const tryItComplete = sawRecording && sawTranscribing;

  const currentStepIndex = useMemo(() => STEPS.findIndex((s) => s.id === stepId), [stepId]);

  async function finish() {
    await api.finishOnboarding();
    navigate("/settings", { replace: true });
  }

  return (
    <div className="mx-auto max-w-2xl p-8">
      <div className="mb-6 flex items-center gap-4">
        <Logo size={56} idSuffix="onboarding" />
        <div>
          <h1 className="text-2xl font-semibold">Welcome to Dicto</h1>
          <p className="text-sm text-ink-400">Hold-to-talk dictation, on your Mac.</p>
        </div>
      </div>

      <Stepper currentIndex={currentStepIndex} />

      <div className="mt-6">
        {stepId === "welcome" && (
          <WelcomeStep onNext={() => setStepId("permissions")} />
        )}
        {stepId === "permissions" && (
          <PermissionsStep
            perms={perms}
            allGranted={allPermsGranted}
            onRequestMic={async () => {
              await api.requestMicrophonePermission();
              await refreshPerms();
            }}
            onBack={() => setStepId("welcome")}
            onNext={() => setStepId("try-it")}
          />
        )}
        {stepId === "try-it" && (
          <TryItStep
            hotkeyChord={hotkeyChord}
            sawRecording={sawRecording}
            sawTranscribing={sawTranscribing}
            complete={tryItComplete}
            onBack={() => setStepId("permissions")}
            onNext={() => setStepId("done")}
          />
        )}
        {stepId === "done" && <DoneStep onFinish={finish} />}
      </div>
    </div>
  );
}

function Stepper({ currentIndex }: { currentIndex: number }) {
  return (
    <ol className="flex items-center gap-2 text-xs">
      {STEPS.map((s, i) => {
        const state =
          i < currentIndex ? "done" : i === currentIndex ? "active" : "pending";
        const pillClass =
          state === "done"
            ? "bg-accent text-white"
            : state === "active"
            ? "border border-accent text-accent"
            : "border border-ink-200 text-ink-400 dark:border-ink-700";
        return (
          <li key={s.id} className="flex items-center gap-2">
            <span
              className={`flex h-6 w-6 items-center justify-center rounded-full text-[10px] ${pillClass}`}
            >
              {i + 1}
            </span>
            <span
              className={state === "active" ? "font-medium" : "text-ink-400"}
            >
              {s.label}
            </span>
            {i < STEPS.length - 1 && <span className="text-ink-300">→</span>}
          </li>
        );
      })}
    </ol>
  );
}

function WelcomeStep({ onNext }: { onNext: () => void }) {
  return (
    <div className="card space-y-4">
      <h2 className="text-lg font-semibold">How Dicto works</h2>
      <ul className="ml-4 list-disc space-y-1 text-sm text-ink-600 dark:text-ink-300">
        <li>Hold your hotkey. Speak. Release.</li>
        <li>Dicto types the cleaned-up text into whatever app you're using.</li>
        <li>The default hotkey is <kbd className="kbd">⌃ Space</kbd> — you can change it in Settings.</li>
        <li>Everything stays on your Mac unless you opt into a cloud cleanup provider.</li>
      </ul>
      <div className="flex justify-end pt-2">
        <button type="button" className="btn-primary" onClick={onNext}>
          Get started
        </button>
      </div>
    </div>
  );
}

function PermissionsStep({
  perms,
  allGranted,
  onRequestMic,
  onBack,
  onNext,
}: {
  perms: PermissionsSnapshot;
  allGranted: boolean;
  onRequestMic: () => Promise<void>;
  onBack: () => void;
  onNext: () => void;
}) {
  return (
    <div className="card space-y-4">
      <div>
        <h2 className="text-lg font-semibold">Grant three macOS permissions</h2>
        <p className="mt-1 text-sm text-ink-500 dark:text-ink-300">
          Status pills update automatically as you grant each in System Settings.
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
      <div className="flex items-center justify-between pt-2">
        <button type="button" className="btn-secondary" onClick={onBack}>
          Back
        </button>
        <button type="button" className="btn-primary" disabled={!allGranted} onClick={onNext}>
          {allGranted ? "Continue" : "Grant all three to continue"}
        </button>
      </div>
    </div>
  );
}

function TryItStep({
  hotkeyChord,
  sawRecording,
  sawTranscribing,
  complete,
  onBack,
  onNext,
}: {
  hotkeyChord: string;
  sawRecording: boolean;
  sawTranscribing: boolean;
  complete: boolean;
  onBack: () => void;
  onNext: () => void;
}) {
  // Auto-advance ~600ms after completion so the user sees the green
  // confirmation before the next step replaces it.
  const advanceRef = useRef(false);
  useEffect(() => {
    if (complete && !advanceRef.current) {
      advanceRef.current = true;
      const id = setTimeout(onNext, 600);
      return () => clearTimeout(id);
    }
    return undefined;
  }, [complete, onNext]);

  return (
    <div className="card space-y-4">
      <div>
        <h2 className="text-lg font-semibold">Try it out</h2>
        <p className="mt-1 text-sm text-ink-500 dark:text-ink-300">
          Press and hold <kbd className="kbd">{hotkeyChord || "your hotkey"}</kbd>, say a sentence, and release. We'll
          verify your mic and hotkey are wired up correctly.
        </p>
      </div>
      <div className="space-y-2 rounded-md border border-ink-200 bg-ink-50 p-4 dark:border-ink-700 dark:bg-ink-800">
        <CheckRow label="Hotkey fired (recording started)" done={sawRecording} />
        <CheckRow label="Audio captured (transcribing)" done={sawTranscribing} />
      </div>
      <div className="flex items-center justify-between pt-2">
        <button type="button" className="btn-secondary" onClick={onBack}>
          Back
        </button>
        <button
          type="button"
          className="btn-primary"
          onClick={onNext}
        >
          {complete ? "Continue" : "Skip"}
        </button>
      </div>
    </div>
  );
}

function CheckRow({ label, done }: { label: string; done: boolean }) {
  return (
    <div className="flex items-center gap-3 text-sm">
      <span
        className={`flex h-5 w-5 items-center justify-center rounded-full text-[11px] ${
          done
            ? "bg-emerald-500 text-white"
            : "border border-ink-300 text-ink-400 dark:border-ink-600"
        }`}
        aria-hidden
      >
        {done ? "✓" : ""}
      </span>
      <span className={done ? "text-ink-700 dark:text-ink-200" : "text-ink-500"}>{label}</span>
    </div>
  );
}

function DoneStep({ onFinish }: { onFinish: () => void }) {
  return (
    <div className="card space-y-4 text-center">
      <div className="text-3xl">✨</div>
      <h2 className="text-lg font-semibold">You're set up</h2>
      <p className="text-sm text-ink-500 dark:text-ink-300">
        Hold your hotkey from anywhere on macOS — Dicto will paste your transcript into whatever
        you're typing in. You can change your hotkey, mic, and cleanup style in Settings any time.
      </p>
      <div className="flex justify-center pt-2">
        <button type="button" className="btn-primary" onClick={onFinish}>
          Open Settings
        </button>
      </div>
    </div>
  );
}
