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
    sublabel: "free, on your Mac",
    description:
      "Cleans up filler words, fixes punctuation, breaks long speech into sentences, and formats lists as bullet points. Uses Apple's on-device writing model.",
    privacy: "Stays on your Mac.",
    speed: "About 300ms.",
  },
  // Hidden from the dropdown until the engine ships (Step 4). The enum value
  // still exists in Rust + TypeScript for forward compatibility.
  bundled_llm: {
    label: "On-device cleanup model",
    sublabel: "free, 940 MB download — coming soon",
    description:
      "Same quality benefits as Apple Intelligence, on any Mac. A small language model that runs entirely on your computer.",
    privacy: "Stays on your Mac.",
    speed: "Half a second to a couple of seconds.",
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
 * `bundled_llm` is hidden until the engine ships in a follow-up.
 * `apple_intelligence` is hidden until the Swift sidecar ships.
 *
 * Update this list as features land; the rest of the metadata is ready.
 */
export const VISIBLE_PROVIDERS: PolishProvider[] = [
  "auto",
  "local_lite",
  "claude",
  "groq_llama",
  "none",
];
