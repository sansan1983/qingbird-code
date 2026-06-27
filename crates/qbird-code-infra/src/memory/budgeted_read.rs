use super::tokenizer::{estimate_tokens_simple, tokens_to_chars};
use super::types::BudgetedReadResult;

pub fn read_budgeted(text: &str, budget_tokens: usize) -> BudgetedReadResult {
    let total_tokens = estimate_tokens_simple(text);
    if total_tokens <= budget_tokens {
        return BudgetedReadResult {
            text: text.to_string(),
            truncated: false,
            total_tokens,
            used_tokens: total_tokens,
        };
    }
    let max_chars = tokens_to_chars(budget_tokens.saturating_sub(5));
    let end = text
        .char_indices()
        .nth(max_chars)
        .map(|(i, _)| i)
        .unwrap_or(text.len());
    let truncated = &text[..end];
    let text = format!("{}\n\n_[truncated, budget exceeded]_", truncated);
    BudgetedReadResult {
        text,
        truncated: true,
        total_tokens,
        used_tokens: budget_tokens,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_read_budgeted_within_budget() {
        let result = read_budgeted("hello world", 100);
        assert!(!result.truncated);
    }
    #[test]
    fn test_read_budgeted_exceeds() {
        let text = "A".repeat(1000);
        let result = read_budgeted(&text, 10);
        assert!(result.truncated);
    }
}
