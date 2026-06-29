use qbird_code_infra::config::{estimate_cost, format_cost};

#[test]
fn test_cost_calculation_simple() {
    let cost = estimate_cost(1000, 500, 0, 2.0, 4.0).unwrap();
    let expected = (1000.0 / 1_000_000.0) * 2.0 + (500.0 / 1_000_000.0) * 4.0;
    assert!((cost - expected).abs() < 1e-10);
}

#[test]
fn test_cost_cache_hit_free() {
    let cost = estimate_cost(1000, 0, 500, 2.0, 0.0).unwrap();
    let expected = (500.0 / 1_000_000.0) * 2.0;
    assert!((cost - expected).abs() < 1e-10);
}

#[test]
fn test_cost_unknown_when_zero() {
    assert!(estimate_cost(1000, 500, 0, 0.0, 0.0).is_none());
}

#[test]
fn test_cost_rmb_conversion() {
    let usd = 0.004;
    let rmb_str = format_cost(usd, true);
    assert_eq!(rmb_str, "≈ ¥0.0288");
}

#[test]
fn test_execute_mode_cost_line() {
    let usd = 0.0012;
    let line = format!("[cost] ${:.4} USD", usd);
    assert_eq!(line, "[cost] $0.0012 USD");
}
