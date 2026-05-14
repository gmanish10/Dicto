pub mod claude;
pub mod groq_llama;
pub mod local_lite;
pub mod noop;
pub mod prompt;
pub mod resolver;

pub use prompt::{build_few_shot_block, build_full_system, SYSTEM_PROMPT};
pub use resolver::{resolve, PolishContext};

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

// Polish system prompt + few-shot helpers live in `prompt.rs`.
// Re-exported above so existing callers keep compiling.
