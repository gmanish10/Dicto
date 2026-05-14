import type { ApiKeyStatus, PolishProvider } from "../lib/ipc";
import { POLISH_META } from "../lib/polishLabels";

interface Props {
  provider: PolishProvider;
  keys: ApiKeyStatus[];
}

/**
 * Plain-language help panel rendered below the polish-provider dropdown.
 *
 * Always shows the same four fields (What it does, Privacy, Speed, Cost)
 * so users can compare options apples-to-apples. The right-side status
 * pill answers "can I use this right now?" — green when ready, yellow
 * when there's setup to do, gray when not yet implemented.
 */
export function PolishProviderHelp({ provider, keys }: Props) {
  const meta = POLISH_META[provider];
  const status = computeStatus(provider, keys);

  return (
    <div className="card mt-2 space-y-3">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="font-medium">{meta.label}</div>
          {meta.sublabel && (
            <div className="text-xs text-ink-400">{meta.sublabel}</div>
          )}
        </div>
        <StatusPill kind={status.kind} text={status.text} />
      </div>

      <Field label="What it does">{meta.description}</Field>
      <Field label="Privacy">{meta.privacy}</Field>
      <Field label="Speed">{meta.speed}</Field>
      {meta.cost && <Field label="Cost">{meta.cost}</Field>}

      {status.action && (
        <div className="pt-1">
          <button
            type="button"
            className="btn-secondary text-xs"
            onClick={status.action.onClick}
          >
            {status.action.label}
          </button>
        </div>
      )}
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="grid grid-cols-[5.5rem_1fr] gap-3 text-sm">
      <div className="text-ink-400">{label}</div>
      <div className="text-ink-700 dark:text-ink-200">{children}</div>
    </div>
  );
}

type StatusKind = "ready" | "needs-setup" | "unavailable";

interface ComputedStatus {
  kind: StatusKind;
  text: string;
  action?: { label: string; onClick: () => void };
}

function StatusPill({ kind, text }: { kind: StatusKind; text: string }) {
  const className =
    kind === "ready"
      ? "pill-green"
      : kind === "needs-setup"
      ? "pill-yellow"
      : "pill bg-ink-200 text-ink-600 dark:bg-ink-700 dark:text-ink-300";
  return <span className={className}>{text}</span>;
}

function computeStatus(
  provider: PolishProvider,
  keys: ApiKeyStatus[]
): ComputedStatus {
  const has = (k: "anthropic" | "groq" | "openai") =>
    keys.find((kk) => kk.key === k)?.configured ?? false;

  switch (provider) {
    case "auto":
    case "local_lite":
    case "none":
      return { kind: "ready", text: "Ready" };

    case "apple_intelligence":
      return {
        kind: "unavailable",
        text: "Coming soon",
      };

    case "bundled_llm":
      return {
        kind: "unavailable",
        text: "Coming soon",
      };

    case "claude":
      return has("anthropic")
        ? { kind: "ready", text: "Ready" }
        : {
            kind: "needs-setup",
            text: "Needs API key",
            action: {
              label: "Get an Anthropic key",
              onClick: () => window.open("https://console.anthropic.com/", "_blank"),
            },
          };

    case "groq_llama":
      return has("groq")
        ? { kind: "ready", text: "Ready" }
        : {
            kind: "needs-setup",
            text: "Needs API key",
            action: {
              label: "Get a Groq key",
              onClick: () => window.open("https://console.groq.com/keys", "_blank"),
            },
          };
  }
}
