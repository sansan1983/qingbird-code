use super::types::TokenInfo;

pub type OverflowLevel = u8;

pub fn usable(input: &TokenInfo) -> usize {
    let reserved = input.reserved.clamp(2000, 8000);
    input
        .model_limit
        .saturating_sub(input.current_usage)
        .saturating_sub(reserved)
}

pub fn overflow_level(input: &TokenInfo) -> OverflowLevel {
    let available = usable(input);
    if available == 0 {
        return 3;
    }
    let ratio = input.current_usage as f64 / available as f64;
    if ratio < 0.50 {
        0
    } else if ratio < 0.70 {
        1
    } else if ratio < 0.85 {
        2
    } else {
        3
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_overflow_safe() {
        let info = TokenInfo {
            model_limit: 64000,
            current_usage: 10000,
            reserved: 4000,
        };
        assert_eq!(overflow_level(&info), 0);
    }
    #[test]
    fn test_overflow_danger() {
        let info = TokenInfo {
            model_limit: 64000,
            current_usage: 55000,
            reserved: 4000,
        };
        assert_eq!(overflow_level(&info), 3);
    }
}
