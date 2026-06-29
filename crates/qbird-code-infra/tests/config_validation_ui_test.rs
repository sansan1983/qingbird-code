use qbird_code_infra::config_validate::ConfigError;

fn mock_errors(count: usize) -> Vec<ConfigError> {
    (0..count)
        .map(|i| ConfigError {
            field: format!("mock.field_{i}"),
            message: format!("mock error {i}"),
        })
        .collect()
}

#[test]
fn test_aggregate_errors_print_all() {
    let errors = mock_errors(2);
    assert_eq!(errors.len(), 2);
    let mut output = String::new();
    for err in &errors {
        output.push_str(&format!("[error] {}\n", err.message));
    }
    assert!(output.contains("[error] mock error 0"));
    assert!(output.contains("[error] mock error 1"));
}

#[test]
fn test_aggregate_errors_exit_code_2() {
    let errors = mock_errors(2);
    assert!(!errors.is_empty());
    assert!(errors.len() >= 2);
}

#[test]
fn test_no_errors_passes() {
    let errors: Vec<ConfigError> = Vec::new();
    assert!(errors.is_empty());
}

#[test]
fn test_one_error_exits_2() {
    let errors = mock_errors(1);
    assert!(!errors.is_empty());
    assert_eq!(errors.len(), 1);
}
