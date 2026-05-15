pub mod prompt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomWord {
    pub id: i64,
    pub word: String,
    pub weight: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replacement {
    pub id: i64,
    pub trigger: String,
    pub replacement: String,
    pub case_sensitive: bool,
}

/// Apply user-defined word replacements to a polished transcript.
/// Replacements run in insertion order (deterministic via id ASC). Triggers are
/// matched as whole words by default; if `case_sensitive` is false, both sides
/// are compared in lowercase.
pub fn apply_replacements(input: &str, replacements: &[Replacement]) -> String {
    let mut text = input.to_string();
    for r in replacements {
        text = replace_whole_word(&text, &r.trigger, &r.replacement, r.case_sensitive);
    }
    text
}

fn replace_whole_word(
    haystack: &str,
    trigger: &str,
    replacement: &str,
    case_sensitive: bool,
) -> String {
    if trigger.is_empty() {
        return haystack.to_string();
    }
    let mut out = String::with_capacity(haystack.len());
    let bytes = haystack.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let remaining = &haystack[i..];
        let matches = if case_sensitive {
            remaining.starts_with(trigger)
        } else {
            remaining
                .get(..trigger.len())
                .map(|s| s.eq_ignore_ascii_case(trigger))
                .unwrap_or(false)
        };
        if matches {
            let before_ok = i == 0
                || !haystack
                    .as_bytes()
                    .get(i - 1)
                    .map(|b| (*b as char).is_alphanumeric())
                    .unwrap_or(false);
            let after_idx = i + trigger.len();
            let after_ok = after_idx >= bytes.len()
                || !haystack
                    .as_bytes()
                    .get(after_idx)
                    .map(|b| (*b as char).is_alphanumeric())
                    .unwrap_or(false);
            if before_ok && after_ok {
                out.push_str(replacement);
                i += trigger.len();
                continue;
            }
        }
        let Some(ch) = remaining.chars().next() else {
            break;
        };
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_whole_word_case_insensitive() {
        let reps = vec![Replacement {
            id: 1,
            trigger: "newline".to_string(),
            replacement: "\n".to_string(),
            case_sensitive: false,
        }];
        assert_eq!(
            apply_replacements("line one Newline line two", &reps),
            "line one \n line two"
        );
    }

    #[test]
    fn does_not_replace_inside_word() {
        let reps = vec![Replacement {
            id: 1,
            trigger: "ml".to_string(),
            replacement: "machine learning".to_string(),
            case_sensitive: false,
        }];
        // "html" contains "ml" but shouldn't be substituted.
        assert_eq!(
            apply_replacements("html and ml", &reps),
            "html and machine learning"
        );
    }

    #[test]
    fn case_sensitive_respected() {
        let reps = vec![Replacement {
            id: 1,
            trigger: "API".to_string(),
            replacement: "API".to_string(),
            case_sensitive: true,
        }];
        // Lowercase "api" should not match the upper-case trigger.
        assert_eq!(apply_replacements("the api", &reps), "the api");
    }
}
