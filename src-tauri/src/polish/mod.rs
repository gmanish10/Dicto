pub mod apple_intelligence;
pub mod bundled_llm;
pub mod claude;
pub mod groq_llama;
pub mod local_lite;
pub mod noop;
pub mod prompt;
pub mod resolver;

pub use prompt::{build_few_shot_block, build_full_system, SYSTEM_PROMPT};
pub use resolver::{
    resolve, try_construct_apple_intelligence, try_construct_bundled_llm, PolishContext,
};

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
    // Refusal detection: only flag when the refusal language appears as
    // the *opening* of the polish, which is where real model refusals
    // live ("I can't help with that.", "As an AI, I cannot…"). Anywhere
    // else in the text is overwhelmingly going to be the speaker
    // actually saying the phrase ("I can't believe how much we covered"),
    // not a refusal. The earlier `contains()` check produced false
    // positives often enough to be a real footgun.
    let lowered = polished.to_ascii_lowercase();
    let head = lowered.get(..lowered.len().min(80)).unwrap_or("");
    for refusal in [
        "i can't help",
        "i cannot help",
        "i'm unable to",
        "i am unable to",
        "as an ai",
        "i don't have the ability",
        "sorry, i can't",
        "sorry, i cannot",
    ] {
        if head.starts_with(refusal) || head.contains(&format!(". {refusal}")) {
            return Err("refusal pattern detected");
        }
    }
    Ok(())
}

// Polish system prompt + few-shot helpers live in `prompt.rs`.
// Re-exported above so existing callers keep compiling.
