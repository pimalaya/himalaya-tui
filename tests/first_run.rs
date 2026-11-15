//! # First run
//!
//! The built binary run without a configuration, its stdin closed and no
//! terminal: it exits 1 naming the path looked at and the sample, as text
//! or as the failure object under `--json`, before the interface would take
//! the terminal.

use std::{
    path::Path,
    process::{Command, Output, Stdio},
};

use serde_json::Value;
use tempfile::TempDir;

/// Runs the binary with `args` in `home`, which holds no configuration of
/// its own, its stdin closed and its log file under `home`.
fn run(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_himalaya-tui"))
        .args(args)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home)
        .env("TMPDIR", home)
        .env_remove("XDG_CONFIG_DIRS")
        .env_remove("RUST_LOG")
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Asserts `output` is the missing configuration at `path`, pointing at
/// the sample, as text or as the failure object.
fn assert_missing(output: &Output, path: &Path) {
    let stdout = text(&output.stdout);

    assert_eq!(output.status.code(), Some(1), "{stdout}");
    let missing = format!("No configuration found at {}", path.display());
    assert!(stdout.contains(&missing), "{stdout}");
    assert!(stdout.contains("config.sample.toml"), "{stdout}");
    assert!(!stdout.contains("Usage:"), "{stdout}");
    assert!(output.stderr.is_empty(), "{}", text(&output.stderr));
    assert!(!path.exists());
}

#[test]
fn a_bare_invocation_without_configuration_names_the_missing_path() {
    let home = TempDir::new().unwrap();
    let path = home.path().join("config.toml");
    let output = run(home.path(), &["-c", path.to_str().unwrap()]);

    assert!(text(&output.stdout).starts_with("Error: "));
    assert_missing(&output, &path);
}

#[test]
fn a_json_caller_without_configuration_gets_the_default_path() {
    let home = TempDir::new().unwrap();
    let path = home.path().join("himalaya").join("config.toml");
    let output = run(home.path(), &["--json"]);

    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json["error"].is_string(), "{json}");
    assert_missing(&output, &path);
}
