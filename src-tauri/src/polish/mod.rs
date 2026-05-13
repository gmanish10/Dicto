pub mod claude;
pub mod groq_llama;
pub mod local_lite;
pub mod noop;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Correction {
    pub raw: String,
    pub final_text: String,
}

#[derive(Error, Debug)]
pub enum PolishError {
    #[error("network error: {0}")]
    Network(String),
    #[error("API error: {0}")]
    Api(String),
    #[error("missing API key")]
    MissingKey,
    #[error("output failed sanity check: {0}")]
    OutputRejected(&'static str),
}

#[async_trait]
pub trait Polisher: Send + Sync {
    async fn polish(
        &self,
        raw: &str,
        recent_corrections: &[Correction],
    ) -> Result<String, PolishError>;

    fn name(&self) -> &'static str;
}

/// Reject obviously-bad LLM output and fall back. Returns Ok(text) if acceptable,
/// Err with reason if not.
pub fn sanity_check(raw: &str, polished: &str) -> Result<(), &'static str> {
    let polished = polished.trim();
    if polished.is_empty() {
        return Err("empty output");
    }
    let raw_len = raw.trim().len().max(1);
    let pol_len = polished.len();
    if pol_len > raw_len * 3 {
        return Err("output is more than 3× the raw length");
    }
    if pol_len < raw_len / 4 && raw_len > 40 {
        return Err("output is less than 25% of the raw length");
    }
    let lowered = polished.to_ascii_lowercase();
    for refusal in [
        "i can't",
        "i cannot",
        "i'm unable",
        "i am unable",
        "as an ai",
        "i don't have",
    ] {
        if lowered.contains(refusal) {
            return Err("refusal pattern detected");
        }
    }
    Ok(())
}

pub(crate) fn build_system_prompt() -> &'static str {
    "You are a transcript polisher. Your job is to clean a raw speech-to-text \
     transcript so that it reads naturally as written text. Apply these rules: \n\
     1. Remove disfluencies: um, uh, like (when used as a filler), you know (as a filler), \
        sort of (as a filler), basically (as a filler), I mean (as a filler).\n\
     2. Remove false starts and repeated words (e.g., \"the the\" → \"the\").\n\
     3. Add appropriate punctuation and capitalization.\n\
     4. Preserve all meaning, technical terms, proper nouns, and the speaker's \
        intent verbatim. Do not add new content or commentary.\n\
     5. If the speaker explicitly dictates punctuation or symbols (e.g., \"period\", \
        \"comma\", \"newline\"), leave them as text; downstream replacements handle these.\n\
     6. Output ONLY the cleaned transcript. No prefacing, no quotation marks around it, \
        no explanation."
}

pub(crate) fn build_few_shot_block(recent: &[Correction]) -> String {
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
