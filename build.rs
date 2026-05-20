use pimalaya_cli::build::{features_env, target_envs};

fn main() {
    features_env(include_str!("./Cargo.toml"));
    target_envs();

    println!("cargo::rustc-env=GIT_DESCRIBE=v0.1.0");
    println!("cargo::rustc-env=GIT_REV=dev");
}
