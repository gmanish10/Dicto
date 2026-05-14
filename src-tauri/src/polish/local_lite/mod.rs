//! Free, offline, heuristic polish — no LLM required.
//!
//! Runs a series of independent, idempotent passes over the raw transcript.
//! Each pass is in its own submodule with table-driven tests so we can iterate
//! on quality without affecting the others.
//!
//! Pass order matters; earlier passes shape what later passes see.
//!
//! ```text
//!  raw
//!   │
//!   ├─► [fillers]     drop "um", "uh", "you know", repeated words
//!   ├─► [contractions] expand "gonna" → "going to", etc.
//!   ├─► [punctuation] capitalize sentence starts, fix standalone "i",
//!   │                 add terminal punctuation, infer question marks
//!   └─► polished text
//! ```
//!
//! **Explicit non-goal:** list/bullet detection. Detecting when a user
//! dictates an enumeration requires semantic understanding; heuristics
//! produce too many false positives ("I went to Alice and Bob and Charlie"
//! isn't a list). The LLM tiers (Apple Intelligence, BundledLlm) handle it.

pub mod contractions;
pub mod fillers;
pub mod punctuation;

#[cfg(test)]
mod tests;

use super::{Correction, PolishError, Polisher};
use async_trait::async_trait;

pub struct LocalLitePolisher;

#[async_trait]
impl Polisher for LocalLitePolisher {
    async fn polish(&self, raw: &str, _recent: &[Correction]) -> Result<String, PolishError> {
        Ok(clean(raw))
    }

    fn name(&self) -> &'static str {
        "local_lite"
    }
}

/// Run the full polish pipeline. Public so the resolver and tests can reach it.
pub fn clean(input: &str) -> String {
    let pass1 = fillers::strip(input);
    let pass2 = contractions::expand(&pass1);
    punctuation::finalize(&pass2)
}
