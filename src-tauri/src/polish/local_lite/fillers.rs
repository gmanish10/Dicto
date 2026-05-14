//! Filler-word removal pass.
//!
//! Two tiers:
//!
//! - **Hard fillers** (single tokens, almost always meaningless):
//!   `um`, `uh`, `umm`, `uhm`, `uhh`, `erm`, `er`, `ah`, `hm`, `hmm`.
//!   Dropped unconditionally.
//!
//! - **Soft fillers** (multi-word phrases, often meaningful but commonly
//!   used as discourse fillers in casual dictation):
//!   `you know`, `i mean`, `sort of`, `kind of`, `basically`.
//!
//! Also collapses adjacent repeated words ("the the" → "the") which Whisper
//! emits during stutters. Cleans up orphan commas left behind by phrase
//! removal ("thing, you know, is" → "thing is").
//!
//! `like` is deliberately excluded — too often meaningful. LLM tiers handle it.

const HARD_FILLERS: &[&str] = &[
    "um", "umm", "uhm", "uh", "uhh", "erm", "er", "ah", "ahh", "hm", "hmm",
];

/// Multi-word soft filler phrases. Each entry is a slice of lowercase tokens
/// that must match consecutively (whole-word, case-insensitive).
const SOFT_FILLER_PHRASES: &[&[&str]] = &[
    &["you", "know"],
    &["i", "mean"],
    &["sort", "of"],
    &["kind", "of"],
    &["basically"],
];

pub fn strip(input: &str) -> String {
    let raw_tokens: Vec<String> = input.split_whitespace().map(str::to_string).collect();
    let lowered: Vec<String> = raw_tokens.iter().map(|t| normalize_token(t)).collect();

    // Pass 1: drop hard fillers and matched soft-filler phrases.
    let mut kept: Vec<String> = Vec::with_capacity(raw_tokens.len());
    let mut i = 0;
    while i < raw_tokens.len() {
        if let Some(phrase_len) = match_soft_filler(&lowered, i) {
            // Trim trailing comma from the preceding kept token so
            // "thing, you know," doesn't become "thing, is".
            if let Some(prev) = kept.last_mut() {
                let trimmed = prev.trim_end_matches(',').to_string();
                if !trimmed.is_empty() {
                    *prev = trimmed;
                }
            }
            i += phrase_len;
            continue;
        }
        if HARD_FILLERS.contains(&lowered[i].as_str()) {
            i += 1;
            continue;
        }
        kept.push(raw_tokens[i].clone());
        i += 1;
    }

    // Pass 2: collapse adjacent repeated words (compare normalized).
    let mut deduped: Vec<String> = Vec::with_capacity(kept.len());
    for token in kept {
        let same = deduped
            .last()
            .map(|p| normalize_token(p) == normalize_token(&token))
            .unwrap_or(false);
        if !same {
            deduped.push(token);
        }
    }

    deduped.join(" ")
}

/// If the tokens starting at `i` match any `SOFT_FILLER_PHRASES`, return the
/// length of the matched phrase. Otherwise None.
fn match_soft_filler(lowered: &[String], i: usize) -> Option<usize> {
    'outer: for phrase in SOFT_FILLER_PHRASES {
        if i + phrase.len() > lowered.len() {
            continue;
        }
        for (j, word) in phrase.iter().enumerate() {
            if lowered[i + j] != *word {
                continue 'outer;
            }
        }
        return Some(phrase.len());
    }
    None
}

/// Lowercase, strip leading/trailing punctuation. Apostrophes within the
/// word are preserved ("don't" stays "don't").
fn normalize_token(t: &str) -> String {
    let chars: Vec<char> = t.chars().collect();
    let start = chars.iter().position(|c| c.is_alphanumeric()).unwrap_or(0);
    let end = chars
        .iter()
        .rposition(|c| c.is_alphanumeric())
        .map(|p| p + 1)
        .unwrap_or(chars.len());
    if start >= end {
        return String::new();
    }
    chars[start..end]
        .iter()
        .collect::<String>()
        .to_ascii_lowercase()
}
