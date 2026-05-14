//! Expand common spoken contractions to written form.
//!
//! Conservative list — only contractions that are unambiguous in dictation
//! context. Avoids "kinda" (could be a casual word in a finished sentence)
//! and "shoulda/woulda/coulda" (often intentional informal voice).

/// (spoken_form, written_form). Comparison is case-insensitive on
/// word boundaries; output preserves the input casing pattern.
const PAIRS: &[(&str, &str)] = &[
    ("gonna", "going to"),
    ("wanna", "want to"),
    ("gotta", "got to"),
    ("dunno", "don't know"),
];

pub fn expand(input: &str) -> String {
    let mut out = Vec::with_capacity(input.split_whitespace().count());
    for token in input.split_whitespace() {
        out.push(expand_one(token));
    }
    out.join(" ")
}

fn expand_one(token: &str) -> String {
    let (prefix, core, suffix) = split_word_boundary(token);
    let lowered = core.to_ascii_lowercase();
    for (spoken, written) in PAIRS {
        if lowered == *spoken {
            return format!("{prefix}{written}{suffix}");
        }
    }
    token.to_string()
}

/// Split a token like "gonna," into ("", "gonna", ","). Anything before the
/// first alphanumeric char becomes `prefix`; anything after the last becomes
/// `suffix`.
fn split_word_boundary(t: &str) -> (String, String, String) {
    let chars: Vec<char> = t.chars().collect();
    let start = chars.iter().position(|c| c.is_alphanumeric()).unwrap_or(0);
    let end = chars
        .iter()
        .rposition(|c| c.is_alphanumeric())
        .map(|p| p + 1)
        .unwrap_or(chars.len());
    if start >= end {
        return (String::new(), t.to_string(), String::new());
    }
    let prefix: String = chars[..start].iter().collect();
    let core: String = chars[start..end].iter().collect();
    let suffix: String = chars[end..].iter().collect();
    (prefix, core, suffix)
}
