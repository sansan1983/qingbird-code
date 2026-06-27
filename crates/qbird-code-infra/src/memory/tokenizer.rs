pub fn estimate_tokens_simple(text: &str) -> usize {
    let mut chinese_chars: usize = 0;
    let mut other_chars: usize = 0;
    for ch in text.chars() {
        if ('\u{4e00}'..='\u{9fff}').contains(&ch) {
            chinese_chars += 1;
        } else {
            other_chars += 1;
        }
    }
    let chinese_tokens = (chinese_chars as f64 * 0.5).ceil() as usize;
    let other_tokens = (other_chars as f64 / 4.0).ceil() as usize;
    chinese_tokens + other_tokens
}

pub fn tokens_to_chars(tokens: usize) -> usize {
    tokens * 3
}
