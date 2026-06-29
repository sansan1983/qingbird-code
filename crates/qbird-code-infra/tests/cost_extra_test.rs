use qbird_code_infra::config::{estimate_cost, format_cost};

#[test]
fn test_cost_only_input_tokens() {
    let cost = estimate_cost(1000, 0, 0, 2.0, 4.0).unwrap();
    let expected = (1000.0 / 1_000_000.0) * 2.0;
    assert!((cost - expected).abs() < 1e-10);
}

#[test]
fn test_cost_only_output_tokens() {
    let cost = estimate_cost(0, 500, 0, 2.0, 4.0).unwrap();
    let expected = (500.0 / 1_000_000.0) * 4.0;
    assert!((cost - expected).abs() < 1e-10);
}

#[test]
fn test_cost_zero_tokens_returns_none_when_unknown() {
    assert!(estimate_cost(0, 0, 0, 0.0, 0.0).is_none());
}

#[test]
fn test_cost_zero_tokens_with_known_rate() {
    let cost = estimate_cost(0, 0, 0, 2.0, 4.0).unwrap();
    assert!((cost - 0.0).abs() < 1e-10);
}

#[test]
fn test_cost_large_token_values() {
    let cost = estimate_cost(10_000_000, 5_000_000, 0, 3.0, 6.0).unwrap();
    let expected = 10.0 * 3.0 + 5.0 * 6.0;
    assert!((cost - expected).abs() < 1e-6);
}

#[test]
fn test_cost_cache_hit_exceeds_input() {
    // Cache hit > input tokens → effective_input = 0 (saturating_sub)
    let cost = estimate_cost(100, 50, 200, 2.0, 4.0).unwrap();
    let expected = (50.0 / 1_000_000.0) * 4.0; // only output cost
    assert!((cost - expected).abs() < 1e-10);
}

#[test]
fn test_format_cost_usd() {
    let s = format_cost(1.2345, false);
    assert_eq!(s, "≈ $1.2345 USD");
}

#[test]
fn test_format_cost_rmb() {
    let s = format_cost(1.0, true);
    assert_eq!(s, "≈ ¥7.2000");
}

#[test]
fn test_format_cost_small_usd() {
    let s = format_cost(0.0001, false);
    assert_eq!(s, "≈ $0.0001 USD");
}
