//! Shared polish system prompt and few-shot construction.
//!
//! Used by every LLM provider (Apple Intelligence sidecar, BundledLlm,
//! Claude, Groq) so polish behavior stays consistent across backends. The
//! Swift helper's `PolishPrompt.swift` is generated from `SYSTEM_PROMPT`
//! at build time (see `build.rs`) to keep Rust and Swift in lockstep.

use super::Correction;

/// Canonical system prompt for transcript polishing.
///
/// Deliberately *minimal-edit*. The model's job is hygiene — strip
/// disfluencies and add punctuation — NOT to rewrite the speaker's
/// voice. v0.2.0 shipped with a prompt that told the model to split
/// run-on sentences into 12-20 word chunks and bulletize any
/// enumeration; users reported it was changing their phrasing and
/// structure. v0.2.1 walks that back: preserve word choice and
/// sentence structure exactly, only fix the things the speech-to-text
/// layer leaves dirty.
///
/// Rules are ordered: earlier ones take precedence. Models will see
/// this verbatim before any user transcript.
pub const SYSTEM_PROMPT: &str = "You are a transcript polisher. The user is dictating; you clean \
the speech-to-text output without rewriting it.

Apply ONLY these minimal edits:

1. FILLERS: Remove filler use of \"um\", \"uh\", \"uhh\", \"erm\", \"ah\", \
\"you know\", \"i mean\". Keep them when they carry meaning (\"I know what \
you mean\", \"I mean it\").

2. FALSE STARTS: Drop immediate stutter repetitions \
(\"the the\" → \"the\", \"I I went\" → \"I went\").

3. PUNCTUATION & CAPS: Add commas, periods, and question marks where the \
speech clearly indicates them. Capitalize sentence starts and the pronoun \
\"I\". End every sentence with terminal punctuation.

4. DICTATED PUNCTUATION: If the speaker says \"period\", \"comma\", \
\"newline\", \"open quote\" etc., leave the literal word in place — \
downstream handles it.

Do NOT:
- Rephrase, reorder, or substitute words. Keep the speaker's exact word \
choice.
- Split or merge sentences. Keep the speaker's sentence structure even if \
sentences run long.
- Convert prose into lists, bullets, or headings.
- Add, remove, summarize, or comment on content.
- Translate or change tone, register, or formality.

Output ONLY the cleaned transcript. No preface, no quotation marks \
wrapping the result, no explanation.";

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

/// Compact system prompt for the Apple Intelligence on-device path
/// (the only caller — see `build_compact_system`). The full
/// `SYSTEM_PROMPT` runs ~500 tokens and dominates the polish budget on
/// the ~3 B Foundation model — prompt processing alone can take
/// 600-800 ms before generation begins. This tighter prompt keeps the
/// must-haves (filler removal, capitalization, punctuation) and adds a
/// light-fluency rule: unlike the strictly minimal-edit `SYSTEM_PROMPT`,
/// it lets the model fix clear grammar errors and smooth genuinely
/// awkward phrasing, while still forbidding content or tone changes.
///
/// Few-shot examples are still appended for personalization.
pub const SYSTEM_PROMPT_COMPACT: &str = "You polish push-to-talk transcripts. Apply these edits:

1. Remove fillers (\"um\", \"uh\", \"you know\", \"i mean\") used as filler. \
Keep them when they carry meaning.
2. Drop stutter repetitions (\"the the\" → \"the\").
3. Add commas, periods, and question marks where speech clearly indicates them.
4. Capitalize sentence starts and \"I\".
5. Fix clear grammar mistakes (verb tense, subject-verb agreement, articles) \
and lightly smooth genuinely awkward or run-on phrasing so it reads naturally. \
Make the smallest change that fixes the problem.

Do NOT add, remove, summarize, or comment on content. Do NOT change the \
meaning. Do NOT change the speaker's tone, register, or formality. Do NOT \
convert prose into lists, bullets, or headings. Keep the speaker's wording \
wherever it already reads naturally — only rephrase what is genuinely awkward.

Output ONLY the cleaned transcript. No preface, no quotation marks, no \
commentary.";

/// Compact system prompt for small on-device models, plus optional
/// few-shot block from the user's recent corrections.
pub fn build_compact_system(recent: &[Correction]) -> String {
    format!("{}{}", SYSTEM_PROMPT_COMPACT, build_few_shot_block(recent))
}
