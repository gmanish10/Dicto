//! Final-pass capitalization, terminal punctuation, and question detection.
//!
//! Runs last so it sees the post-filler, post-contraction stream.
//!
//! Steps:
//! 1. Standalone `i` (case-insensitive, surrounded by non-letters) → `I`.
//! 2. Sentence-initial capitalization: the very first letter of the string,
//!    and the first letter after every `.`, `?`, or `!` followed by a space.
//! 3. Question detection: if the text starts with an interrogative word
//!    AND has no `?` already, swap the terminal `.` for `?` (or add `?`).
//! 4. Terminal punctuation: if the result doesn't end in
//!    `. ? ! , ; :`, append `.`.

/// Words that, when first in a sentence, strongly suggest the sentence is
/// a question. Conservative — only the unambiguous ones.
const INTERROGATIVES: &[&str] = &[
    "what", "why", "how", "when", "where", "who", "is", "are", "do", "does", "did", "can", "could",
    "would", "should", "will",
];

pub fn finalize(input: &str) -> String {
    let s = fix_standalone_i(input);
    let s = capitalize_sentence_starts(&s);
    let s = ensure_question_mark(&s);
    add_terminal_period_if_missing(&s)
}

fn fix_standalone_i(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut prev_alpha = false;
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if (c == 'i' || c == 'I') && !prev_alpha {
            // Next char must also not be alphanumeric / apostrophe (so
            // we don't capitalize the "I" in "isn't" or "is").
            let next_is_non_word = chars
                .get(i + 1)
                .map(|n| !n.is_alphanumeric() && *n != '\'')
                .unwrap_or(true);
            if next_is_non_word {
                out.push('I');
                prev_alpha = true;
                i += 1;
                continue;
            }
        }
        out.push(c);
        prev_alpha = c.is_alphanumeric() || c == '\'';
        i += 1;
    }
    out
}

fn capitalize_sentence_starts(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let mut next_capital = true;
    for (i, &c) in chars.iter().enumerate() {
        if next_capital && c.is_alphabetic() {
            out.extend(c.to_uppercase());
            next_capital = false;
        } else {
            out.push(c);
            // After a sentence terminator + whitespace, capitalize next.
            if matches!(c, '.' | '?' | '!') {
                // Find the next non-whitespace char; if exists, mark it.
                if chars.get(i + 1).is_some_and(|n| n.is_whitespace()) {
                    next_capital = true;
                }
            }
        }
    }
    out
}

/// Walk the input one sentence at a time (split on `. `, `? `, `! `,
/// or end-of-input). For each sentence whose first word is an
/// interrogative AND doesn't already end with `?`, swap the terminator
/// for `?` (or append one).
fn ensure_question_mark(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    if chars.is_empty() {
        return String::new();
    }

    let mut out = String::with_capacity(input.len());
    let mut sentence_start = 0;
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        let is_terminator = matches!(c, '.' | '?' | '!');
        // A terminator ends a sentence when followed by whitespace OR end-of-input.
        let ends_sentence = is_terminator
            && (i + 1 == chars.len() || chars.get(i + 1).is_some_and(|n| n.is_whitespace()));
        let at_end_no_terminator = i + 1 == chars.len() && !is_terminator;

        if ends_sentence || at_end_no_terminator {
            // Inclusive end of sentence content (the terminator if present).
            let s_end = i + 1;
            let sentence: String = chars[sentence_start..s_end].iter().collect();
            out.push_str(&maybe_swap_to_question(&sentence));
            sentence_start = s_end;
        }
        i += 1;
    }

    out
}

/// Apply the interrogative-first-word swap to a single sentence.
fn maybe_swap_to_question(sentence: &str) -> String {
    let trimmed = sentence.trim_start();
    let leading_ws_len = sentence.len() - trimmed.len();
    let leading: String = sentence.chars().take_while(|c| c.is_whitespace()).collect();
    let _ = leading_ws_len;

    if trimmed.is_empty() || trimmed.contains('?') {
        return sentence.to_string();
    }
    let first_word: String = trimmed
        .split_whitespace()
        .next()
        .unwrap_or("")
        .chars()
        .filter(|c| c.is_alphabetic())
        .collect::<String>()
        .to_ascii_lowercase();
    if !INTERROGATIVES.contains(&first_word.as_str()) {
        return sentence.to_string();
    }
    // Swap the trailing terminator. Preserve any trailing whitespace.
    let trimmed_end = trimmed.trim_end();
    let trailing_ws: String = trimmed
        .chars()
        .rev()
        .take_while(|c| c.is_whitespace())
        .collect();
    let trailing_ws: String = trailing_ws.chars().rev().collect();
    let mut body = trimmed_end.to_string();
    if body.ends_with('.') {
        body.pop();
        body.push('?');
    } else if !body.ends_with(['?', '!']) {
        body.push('?');
    }
    format!("{leading}{body}{trailing_ws}")
}

fn add_terminal_period_if_missing(input: &str) -> String {
    let trimmed = input.trim_end();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.ends_with(['.', '?', '!', ',', ';', ':']) {
        return trimmed.to_string();
    }
    format!("{trimmed}.")
}
