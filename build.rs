use std::env;

use pimalaya_cli::build::{features_env, git_envs, target_envs};

fn main() {
    features_env(include_str!("./Cargo.toml"));
    target_envs();
    git_envs();

    // `backend` is set when any backend feature is on, SMTP included: what
    // himalaya's merged account context needs to exist.
    println!("cargo::rustc-check-cfg=cfg(backend)");

    let backend = [
        "IMAP", "SMTP", "JMAP", "GMAIL", "MSGRAPH", "MAILDIR", "MBOX", "PIMDIR",
    ]
    .iter()
    .any(|feature| env::var_os(format!("CARGO_FEATURE_{feature}")).is_some());

    if backend {
        println!("cargo::rustc-cfg=backend");
    }
}
