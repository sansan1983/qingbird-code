use std::process::Command;

/// Helper: run the binary with given args, capture output
fn qingbird(args: &[&str]) -> (String, String, Option<i32>) {
    // Binary is sibling of the test runner in target/debug/
    let bin = {
        let exe = std::env::current_exe().expect("current exe");
        let parent = exe.parent().unwrap(); // deps/ or debug/
        let debug_dir = if parent.ends_with("deps") {
            parent.parent().unwrap()
        } else {
            parent
        };
        let bin_name = if cfg!(windows) {
            "qingbird.exe"
        } else {
            "qingbird"
        };
        debug_dir.join(bin_name)
    };

    let output = Command::new(&bin)
        .args(args)
        .output()
        .expect("failed to run qingbird binary");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code();
    (stdout, stderr, code)
}

#[test]
fn test_execute_without_config_shows_usage() {
    // Without any config, --execute should exit with error
    let (stdout, stderr, code) = qingbird(&["--execute", "hello"]);
    // Should fail: no valid config or API key
    assert!(
        code != Some(0),
        "expected non-zero exit, got {code:?}\nstdout: {stdout}\nstderr: {stderr}"
    );
    // Should not panic - some error message expected
    assert!(!stderr.is_empty() || !stdout.is_empty());
}

#[test]
fn test_no_args_shows_usage() {
    let (_stdout, stderr, code) = qingbird(&[]);
    // No args should show error (no subcommand specified)
    assert_eq!(code, Some(1));
    assert!(stderr.contains("Usage") || stderr.contains("qingbird"));
}

#[test]
fn test_help_succeeds() {
    let (stdout, stderr, code) = qingbird(&["--help"]);
    assert_eq!(code, Some(0), "stdout: {stdout}\nstderr: {stderr}");
    assert!(stdout.contains("qingbird"));
    assert!(stdout.contains("--execute"));
    assert!(stdout.contains("--interactive"));
}

#[test]
fn test_version_output() {
    let (stdout, _, code) = qingbird(&["--version"]);
    assert_eq!(code, Some(0));
    assert!(stdout.contains("0.3.0"));
}
