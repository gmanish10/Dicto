/**
 * Plain-language labels + help copy for each polish provider.
 *
 * Kept in one place so:
 * 1. Non-technical strings are easy to audit / localize.
 * 2. The dropdown, help panel, About page, and toasts share wording.
 * 3. Adding/renaming a provider is a one-file change.
 */

import type { PolishProvider } from "./ipc";

export interface ProviderMeta {
  /** Short label shown in the dropdown. No jargon. */
  label: string;
  /** Sentence shown next to the label as a sub-line (e.g., for download size, "needs API key"). */
  sublabel?: string;
  /** Plain-English description of what this provider does. */
  description: string;
  /** Privacy claim — one line. */
  privacy: string;
  /** Typical latency (in user-facing terms). */
  speed: string;
  /** Optional cost note. */
  cost?: string;
  /** Whether this provider needs no setup ("Always available") or has setup steps. */
  alwaysReady?: boolean;
}

export const POLISH_META: Record<PolishProvider, ProviderMeta> = {
  auto: {
    label: "Best available",
    sublabel: "recommended — Dicto picks for you",
    description:
      "Picks the best free cleanup option that works on your Mac. Falls back automatically if something isn't available — you never end up with no cleanup.",
    privacy: "Stays on your Mac whenever possible.",
    speed: "Same as whichever option is in use.",
    alwaysReady: true,
  },
  apple_intelligence: {
    label: "Apple Intelligence",
    sublabel: "free, on your Mac — macOS 26+",
    description:
      "Cleans up filler words, fixes punctuation, and breaks long speech into sentences. Tries to format obvious lists as bullets but doesn't always — Apple's on-device model is small and trades some output quality for being free and fully private. For sharper bullet/heading formatting, use Claude.",
    privacy: "Stays on your Mac. Nothing sent to any server.",
    speed: "About 1.5–2 seconds for short transcripts.",
  },
  // Hidden from the dropdown for v0.2.0: the Rust/UI plumbing is complete,
  // but llama-cpp-2 0.1.146 on macOS 26 (Tahoe) miscompiles a tensor read
  // and rejects valid GGUFs as "duplicated", which blocks the feature at
  // runtime. The Foundation Models option (`apple_intelligence`) covers
  // the same use case natively on macOS 26 and is shipping in its place.
  // Revisit when upstream llama-cpp-2 fixes the Tahoe regression.
  bundled_llm: {
    label: "On-device cleanup model",
    sublabel: "coming in a later release",
    description:
      "Smart cleanup using Qwen 2.5, a small language model that runs entirely on your computer. Removes filler words, fixes punctuation, breaks long speech into sentences, and formats lists as bullet points when appropriate.",
    privacy: "Stays on your Mac. Nothing sent to any server.",
    speed: "Half a second to two seconds depending on the transcript length.",
  },
  local_lite: {
    label: "Basic cleanup",
    sublabel: "free, on your Mac",
    description:
      "Removes \"um\", \"uh\", false starts, and repeated words. Fixes capitalization and adds basic punctuation. No grammar rewriting or list formatting.",
    privacy: "Stays on your Mac. No internet needed.",
    speed: "Instant.",
    alwaysReady: true,
  },
  claude: {
    label: "Claude Haiku",
    sublabel: "high quality, needs your Anthropic API key",
    description:
      "Cloud cleanup using Anthropic's Claude Haiku model. Best output quality for messy transcripts and structured speech.",
    privacy:
      "Each transcript is sent to Anthropic's servers when polishing. Their privacy policy applies.",
    speed: "About half a second to a second.",
    cost: "Pennies per hour of dictation, billed by Anthropic.",
  },
  groq_llama: {
    label: "Groq Llama",
    sublabel: "fast cloud cleanup, needs your Groq API key",
    description:
      "Cloud cleanup using Groq's Llama model. Very fast; quality slightly below Claude.",
    privacy:
      "Each transcript is sent to Groq's servers when polishing. Their privacy policy applies.",
    speed: "About 100-300ms.",
    cost: "Free tier covers most personal use; otherwise pennies per hour.",
  },
  none: {
    label: "No cleanup",
    description:
      "Paste exactly what was transcribed, fillers and all. Useful if you want raw transcripts or want to edit by hand.",
    privacy: "Stays on your Mac. No cleanup happens.",
    speed: "Instant.",
    alwaysReady: true,
  },
};

/**
 * Order of options in the Settings dropdown. Auto comes first as the
 * recommended default. Providers not yet implemented are filtered out by
 * the component using `VISIBLE_PROVIDERS`.
 */
export const PROVIDER_ORDER: PolishProvider[] = [
  "auto",
  "apple_intelligence",
  "bundled_llm",
  "local_lite",
  "claude",
  "groq_llama",
  "none",
];

/**
 * Subset of providers shown in the Settings dropdown right now.
 *
 * Hidden:
 * - `bundled_llm` for v0.2.0 — runtime blocked by an upstream llama-cpp-2
 *   regression on macOS 26 Tahoe (#4 will reopen when that's fixed).
 *
 * Update this list as features land; the rest of the metadata is ready.
 */
export const VISIBLE_PROVIDERS: PolishProvider[] = [
  "auto",
  "apple_intelligence",
  "local_lite",
  "claude",
  "groq_llama",
  "none",
];
