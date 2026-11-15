//! # Exit contract
//!
//! The built binary as a script meets it, its stdin closed and no terminal:
//! a command line the grammar refuses exits 2, its failure object coded
//! `usage-invalid` under `--json`, and a standard command missing its
//! argument fails on stdout, as in himalaya. None of them opens the
//! interface, which needs a terminal.

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

#[test]
fn a_refused_command_line_under_json_prints_the_usage_failure_object() {
    let home = TempDir::new().unwrap();
    let output = run(home.path(), &["--json", "--nope"]);
    let stdout = text(&output.stdout);

    assert_eq!(output.status.code(), Some(2), "{stdout}");
    let json: Value = serde_json::from_str(&stdout).unwrap();
    let error = json["error"].as_str().unwrap();
    assert_eq!(json["code"], "usage-invalid", "{json}");
    assert!(error.starts_with("unexpected argument '--nope'"), "{error}");
    assert_eq!(json["sources"], Value::Array(Vec::new()), "{json}");
    assert_eq!(json["backtrace"], Value::Null, "{json}");
    assert!(output.stderr.is_empty(), "{}", text(&output.stderr));
}

#[test]
fn a_refused_subcommand_line_under_json_prints_the_usage_failure_object() {
    let home = TempDir::new().unwrap();
    let output = run(home.path(), &["--json", "manual", "--nope"]);
    let stdout = text(&output.stdout);

    assert_eq!(output.status.code(), Some(2), "{stdout}");
    let json: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["code"], "usage-invalid", "{json}");
    assert!(output.stderr.is_empty(), "{}", text(&output.stderr));
}

#[test]
fn a_refused_command_line_without_json_is_clap_rendering_on_stderr() {
    let home = TempDir::new().unwrap();
    let output = run(home.path(), &["--nope"]);
    let stderr = text(&output.stderr);

    assert_eq!(output.status.code(), Some(2), "{stderr}");
    assert!(output.stdout.is_empty(), "{}", text(&output.stdout));
    assert!(stderr.starts_with("error: unexpected argument"), "{stderr}");
}

/// A `--json` after `--` is a positional, not the flag.
#[test]
fn a_json_word_after_the_separator_leaves_the_refusal_on_stderr() {
    let home = TempDir::new().unwrap();
    let output = run(home.path(), &["--nope", "--", "--json"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty(), "{}", text(&output.stdout));
    assert!(!output.stderr.is_empty());
}

/// `completion` and `manual` with no argument fail as himalaya's do, the
/// error on stdout rather than their help, uncoded under `--json`.
#[test]
fn a_standard_command_without_its_argument_fails_on_stdout() {
    let home = TempDir::new().unwrap();

    for command in ["completion", "manual"] {
        let output = run(home.path(), &[command]);
        let stdout = text(&output.stdout);

        assert_eq!(output.status.code(), Some(1), "{command}: {stdout}");
        assert!(stdout.starts_with("Error: Writing several"), "{stdout}");
        assert!(stdout.contains("use --dir"), "{command}: {stdout}");
        assert!(!stdout.contains("Usage:"), "{command}: {stdout}");
        assert!(output.stderr.is_empty(), "{}", text(&output.stderr));

        let output = run(home.path(), &["--json", command]);
        let json: Value = serde_json::from_slice(&output.stdout).unwrap();

        assert_eq!(output.status.code(), Some(1), "{command}: {json}");
        assert_eq!(json["code"], Value::Null, "{json}");
        assert!(json["error"].as_str().unwrap().contains("use --dir"));
        assert!(output.stderr.is_empty(), "{}", text(&output.stderr));
    }
}

/// The help lists the standard commands in himalaya's order, with the
/// abouts pimalaya-cli gives them.
#[test]
fn the_standard_commands_read_as_in_himalaya() {
    let home = TempDir::new().unwrap();
    let output = run(home.path(), &["--help"]);
    let stdout = text(&output.stdout);

    assert!(output.status.success(), "{stdout}");
    let commands: Vec<&str> = stdout
        .lines()
        .skip_while(|line| *line != "Commands:")
        .skip(1)
        .take_while(|line| !line.is_empty())
        .map(str::trim)
        .collect();

    let [completion, manual, help] = commands[..] else {
        panic!("{stdout}");
    };
    let about = "Generate completion script for the given shell(s)";
    assert!(completion.starts_with("completion "), "{completion}");
    assert!(completion.ends_with(about), "{completion}");
    let about = "Generate manual page for the given command(s)";
    assert!(manual.starts_with("manual "), "{manual}");
    assert!(manual.ends_with(about), "{manual}");
    assert!(help.starts_with("help "), "{help}");
}
