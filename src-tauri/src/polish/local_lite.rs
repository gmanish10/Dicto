use super::{Correction, PolishError, Polisher};
use async_trait::async_trait;

/// Free, offline filler/disfluency stripper. Handles the common cases that don't
/// need an LLM — "um", "uh", and immediately-repeated words. Also tidies
/// whitespace and capitalizes the first letter.
///
/// This is what the user gets by default before they configure an LLM polish key.
pub struct LocalLitePolisher;

// Words that count as fillers when they appear alone. Conservative list — we
// want to avoid eating meaningful content.
const FILLER_TOKENS: &[&str] = &["um", "uh", "umm", "uhh", "uhm", "erm", "er", "ah"];

#[async_trait]
impl Polisher for LocalLitePolisher {
    async fn polish(&self, raw: &str, _recent: &[Correction]) -> Result<String, PolishError> {
        Ok(clean(raw))
    }

    fn name(&self) -> &'static str {
        "local_lite"
    }
}

pub fn clean(input: &str) -> String {
    // Step 1: drop filler tokens.
    let kept: Vec<&str> = input
        .split_whitespace()
        .filter(|t| {
            let stripped: String = t
                .chars()
                .filter(|c| c.is_alphabetic())
                .collect::<String>()
                .to_ascii_lowercase();
            !FILLER_TOKENS.contains(&stripped.as_str())
        })
        .collect();

    // Step 2: collapse immediately-repeated words (case-insensitive).
    let mut deduped: Vec<&str> = Vec::with_capacity(kept.len());
    for token in kept {
        let prev = deduped.last().copied();
        let same = prev.map(|p| p.eq_ignore_ascii_case(token)).unwrap_or(false);
        if !same {
            deduped.push(token);
        }
    }
    let joined = deduped.join(" ");

    // Step 3: capitalize the first letter.
    let mut chars = joined.chars();
    let result: String = match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    };

    // Step 4: ensure terminal punctuation.
    if !result.is_empty() && !result.ends_with(['.', '?', '!', ',', ';', ':']) {
        format!("{result}.")
    } else {
        result
    }
}

#[cfg(test)]
mod tests {
    use super::clean;

    #[test]
    fn removes_ums_and_uhs() {
        let out = clean("um so like uh the the thing is uh we should ship it tomorrow");
        // "like" is a soft filler — we leave it alone to be safe.
        assert!(!out.to_lowercase().contains("um "));
        assert!(!out.to_lowercase().contains(" uh "));
        assert!(!out.to_lowercase().contains("uh "));
        assert!(out.starts_with("So"));
        assert!(out.ends_with('.'));
        assert!(!out.contains("the the"));
    }

    #[test]
    fn already_clean_passes_through() {
        let out = clean("Hello world");
        assert_eq!(out, "Hello world.");
    }

    #[test]
    fn collapses_repeated_word_case_insensitively() {
        let out = clean("the The thing");
        assert_eq!(out, "The thing.");
    }
}
