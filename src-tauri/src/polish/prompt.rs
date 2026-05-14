//! Shared polish system prompt and few-shot construction.
//!
//! Used by every LLM provider (Apple Intelligence sidecar, BundledLlm,
//! Claude, Groq) so polish behavior stays consistent across backends. The
//! Swift helper's `PolishPrompt.swift` is generated from `SYSTEM_PROMPT`
//! at build time (see `build.rs`) to keep Rust and Swift in lockstep.

use super::Correction;

/// Canonical system prompt for transcript polishing.
///
/// Rules are ordered: earlier ones take precedence. Models will see this
/// verbatim before any user transcript.
pub const SYSTEM_PROMPT: &str =
    "You are a transcript polisher. Convert a raw push-to-talk speech-to-text \
transcript into clean written text.

CRITICAL — LIST FORMATTING (apply this first when relevant):
When the speaker enumerates items — using ordinals (\"first … second … third\"), \
counts (\"three things\", \"two reasons\"), or any sequence of comparable items — \
output a markdown bulleted list with one item per line. Each line MUST start \
with \"- \" (dash space). Do not collapse the list back into prose.

Example:
   Raw: we need to discuss three things first the budget then the timeline \
then the hiring plan
   Polished:
   We need to discuss three things:
   - The budget
   - The timeline
   - The hiring plan

Do NOT bulletize a normal two-item conjunction (\"apples and oranges\" stays \
as prose).

Then apply these rules:

1. FILLERS: Remove \"um\", \"uh\", \"uhh\", \"erm\", \"ah\", \"you know\", \"i mean\", \
\"sort of\", \"kind of\", \"basically\", \"like\" — but ONLY when used as filler, \
NOT when they carry meaning (\"I like pizza\", \"sort of like X\").

2. FALSE STARTS: Drop immediate restarts and stutter repetitions \
(\"the the\" → \"the\", \"I I went\" → \"I went\").

3. PUNCTUATION & CAPS: Add commas, periods, question marks, and proper \
capitalization. Capitalize \"I\". End every sentence with terminal punctuation.

4. SENTENCE BREAKING: Long run-on speech becomes multiple short sentences. \
Break on natural clause boundaries; prefer 12-20 word sentences.

5. PRESERVE meaning verbatim. Do NOT add content, opinions, or commentary. \
Keep proper nouns, technical jargon, numbers, and units intact.

6. DICTATED PUNCTUATION: If the speaker says \"period\", \"comma\", \"newline\", \
\"open quote\" etc., leave the literal word in place — downstream handles it.

7. OUTPUT ONLY the cleaned text. No preface, no quotation marks wrapping \
the result, no explanation, no \"here is the polished version\".";

/// Build a few-shot block from the user's recent accepted polish corrections.
/// Returns an empty string when there are no examples to inject.
///
/// The format mirrors what the user is implicitly teaching us: paired
/// raw → final transformations.
pub fn build_few_shot_block(recent: &[Correction]) -> String {
    if recent.is_empty() {
        return String::new();
    }
    let mut s =
        String::from("\n\nExamples of how this user previously polished their transcripts:\n");
    for c in recent.iter().take(5) {
        s.push_str(&format!(
            "- Raw: {}\n  Final: {}\n",
            c.raw.replace('\n', " ").trim(),
            c.final_text.replace('\n', " ").trim()
        ));
    }
    s
}

/// Convenience: full system prompt (rules + few-shot block).
pub fn build_full_system(recent: &[Correction]) -> String {
    format!("{}{}", SYSTEM_PROMPT, build_few_shot_block(recent))
}

/// Compact system prompt for small on-device models (Apple Intelligence,
/// bundled LLM). The full `SYSTEM_PROMPT` runs ~500 tokens and dominates
/// the polish budget on a ~3 B parameter model — prompt processing alone
/// can take 600–800 ms before generation begins. This tighter prompt
/// keeps the must-haves (fillers, capitalization, bullet lists) and
/// drops the marginal rules.
///
/// Few-shot examples are still appended for personalization.
pub const SYSTEM_PROMPT_COMPACT: &str =
    "You polish push-to-talk transcripts. Output ONLY the cleaned text.

Rules:
1. Remove fillers (\"um\", \"uh\", \"you know\", \"i mean\", \"like\" used as filler).
2. Fix capitalization and add punctuation.
3. Drop false starts and stutter repetitions.
4. If the speaker enumerates items (\"first … second … third\", \"three things are …\"), format the items as a markdown bulleted list with each line starting with \"- \".
5. Preserve meaning, proper nouns, and numbers exactly.

Example:
Raw: we need to discuss three things first the budget then the timeline then the hiring plan
Polished:
We need to discuss three things:
- The budget
- The timeline
- The hiring plan";

/// Compact system prompt for small on-device models, plus optional
/// few-shot block from the user's recent corrections.
pub fn build_compact_system(recent: &[Correction]) -> String {
    format!("{}{}", SYSTEM_PROMPT_COMPACT, build_few_shot_block(recent))
}
