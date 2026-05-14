pub mod paste;
pub mod target;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum InjectError {
    #[error("clipboard error: {0}")]
    Clipboard(String),
    #[error("event injection error: {0}")]
    Event(String),
    #[error("secure input mode is active — text copied to clipboard instead")]
    SecureInputActive,
}

pub trait Injector: Send + Sync {
    fn inject(&self, text: &str) -> Result<(), InjectError>;
}

/// Add a single trailing space or newline to the polished output so the
/// user can immediately keep dictating without manual cursor work:
///
/// - When the text ends in sentence-terminating punctuation (`.`, `!`,
///   `?`, `:`), append a space — so the next sentence the user dictates
///   doesn't run into the previous one.
/// - When the last non-empty line is a bullet (`- foo`) or numbered
///   item (`1. foo`), append a newline — so the next dictation lands
///   on a fresh bullet/line rather than continuing the current item.
/// - Otherwise return the text unchanged (e.g., mid-clause output
///   ending in a comma).
///
/// This is purely a "next-edit ergonomics" helper. The stored history
/// transcript keeps the original polished text so reads stay clean.
pub fn format_for_injection(text: &str) -> String {
    let trimmed = text.trim_end_matches([' ', '\t']);
    if trimmed.is_empty() {
        return String::new();
    }

    let last_line = trimmed.lines().last().unwrap_or("").trim_start();
    let is_bullet = last_line.starts_with("- ")
        || last_line.starts_with("* ")
        || numbered_list_prefix(last_line);

    if is_bullet {
        // Don't double up if the polish already ended with a newline.
        if trimmed.ends_with('\n') {
            return trimmed.to_string();
        }
        return format!("{trimmed}\n");
    }

    if let Some(last_char) = trimmed.chars().last() {
        if matches!(last_char, '.' | '!' | '?' | ':' | ';') {
            return format!("{trimmed} ");
        }
        // Closing-quote-after-punctuation patterns (`"`, `'`, `”`, `’`).
        if matches!(last_char, '"' | '\'' | '\u{201d}' | '\u{2019}') {
            let mut chars = trimmed.chars().rev();
            chars.next(); // skip the quote
            if let Some(prev) = chars.next() {
                if matches!(prev, '.' | '!' | '?') {
                    return format!("{trimmed} ");
                }
            }
        }
    }

    trimmed.to_string()
}

/// True when the line begins with `<digits>. ` or `<digits>) ` — i.e., a
/// numbered list item. Used to decide whether to append a newline after
/// the polished output.
fn numbered_list_prefix(line: &str) -> bool {
    let mut chars = line.chars();
    let mut saw_digit = false;
    while let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            saw_digit = true;
            continue;
        }
        if !saw_digit {
            return false;
        }
        if c == '.' || c == ')' {
            return matches!(chars.next(), Some(' '));
        }
        return false;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::format_for_injection;

    #[test]
    fn appends_space_after_terminal_punctuation() {
        assert_eq!(format_for_injection("Hello world."), "Hello world. ");
        assert_eq!(format_for_injection("Done!"), "Done! ");
        assert_eq!(format_for_injection("Really?"), "Really? ");
        assert_eq!(format_for_injection("Note:"), "Note: ");
    }

    #[test]
    fn appends_space_after_quoted_sentence() {
        assert_eq!(
            format_for_injection("She said \"hello.\""),
            "She said \"hello.\" "
        );
    }

    #[test]
    fn appends_newline_after_bullet_line() {
        let s = "We need to discuss:\n- The budget\n- The timeline";
        let expected = "We need to discuss:\n- The budget\n- The timeline\n";
        assert_eq!(format_for_injection(s), expected);
    }

    #[test]
    fn appends_newline_after_numbered_line() {
        let s = "Steps:\n1. Install\n2. Configure";
        let expected = "Steps:\n1. Install\n2. Configure\n";
        assert_eq!(format_for_injection(s), expected);
    }

    #[test]
    fn no_change_when_ending_mid_clause() {
        assert_eq!(format_for_injection("Hello world"), "Hello world");
        assert_eq!(format_for_injection("This is, however"), "This is, however");
    }

    #[test]
    fn idempotent_for_text_already_with_newline() {
        let s = "- A\n- B\n";
        assert_eq!(format_for_injection(s), "- A\n- B\n");
    }

    #[test]
    fn empty_input_returns_empty() {
        assert_eq!(format_for_injection(""), "");
        assert_eq!(format_for_injection("   "), "");
    }
}
