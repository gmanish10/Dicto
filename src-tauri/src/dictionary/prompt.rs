use super::CustomWord;

/// whisper.cpp's `--prompt` budget is roughly 224 tokens. We reserve a small
/// preamble, then concatenate user-supplied vocabulary in weight-descending
/// order until we approach the budget.
///
/// Rule of thumb: ~4 chars per token (GPT-2 BPE on plain English).
const TOKEN_BUDGET: usize = 220;
const CHARS_PER_TOKEN: usize = 4;
const PREAMBLE: &str = "Vocabulary used by the speaker: ";

pub fn build(words: &[CustomWord]) -> String {
    if words.is_empty() {
        return String::new();
    }

    let mut sorted = words.to_vec();
    sorted.sort_by(|a, b| b.weight.cmp(&a.weight).then(a.id.cmp(&b.id)));

    let mut out = String::from(PREAMBLE);
    let mut tokens_used = PREAMBLE.len() / CHARS_PER_TOKEN + 1;

    for (idx, w) in sorted.iter().enumerate() {
        let cost = w.word.len() / CHARS_PER_TOKEN + 2; // word + comma+space
        if tokens_used + cost > TOKEN_BUDGET {
            break;
        }
        if idx > 0 {
            out.push_str(", ");
        }
        out.push_str(&w.word);
        tokens_used += cost;
    }
    out.push('.');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(id: i64, word: &str, weight: i64) -> CustomWord {
        CustomWord {
            id,
            word: word.to_string(),
            weight,
            created_at: 0,
        }
    }

    #[test]
    fn empty_returns_empty() {
        assert_eq!(build(&[]), "");
    }

    #[test]
    fn weight_dominates_order() {
        let prompt = build(&[w(1, "alpha", 1), w(2, "beta", 10), w(3, "gamma", 5)]);
        assert!(prompt.contains("beta, gamma, alpha"));
    }

    #[test]
    fn respects_token_budget() {
        // 100 long words → most must be dropped.
        let many: Vec<CustomWord> = (0..100).map(|i| w(i, &"x".repeat(60), 1)).collect();
        let prompt = build(&many);
        assert!(prompt.len() / CHARS_PER_TOKEN < TOKEN_BUDGET + 10);
    }
}
