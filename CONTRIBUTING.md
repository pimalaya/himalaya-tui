# Contributing guide

Thank you for investing your time in contributing to Himalaya TUI.

## Development environment

The development environment is managed by [Nix flakes](https://nixos.wiki/wiki/Flakes). Running `nix develop` (or `nix-shell` for non-flake users) spawns a shell with the right Rust toolchain, `cargo-deny`, `pkg-config` and the OpenSSL / DBus libraries.

If you do not want to use Nix, install [rustup](https://rust-lang.github.io/rustup/index.html) and pull the toolchain pinned by `rust-version` in `Cargo.toml`:

```
rustup update
```

- `cargo` (>= `v1.87`)
- `rustc` (>= `v1.87`, edition 2024)

## Build

```
cargo build
```

You can disable default [features](https://doc.rust-lang.org/cargo/reference/features.html) with `--no-default-features` and enable individual features with `--features feat1,feat2`.

For example, an IMAP+SMTP-only release build:

```
cargo build --no-default-features --features imap,smtp,rustls-ring --release
```

## Project layout

Himalaya TUI is a thin terminal front-end on top of the [Pimalaya](https://github.com/pimalaya) libraries; the same backend code powers the [`himalaya`](https://github.com/pimalaya/himalaya) CLI. Most of the work happens in companion crates rather than in this repository:

- [io-email](https://github.com/pimalaya/io-email): cross-protocol email client (`EmailClientStd`, shared `Envelope` / `Mailbox` / `Flag` types).
- [io-imap](https://github.com/pimalaya/io-imap), [io-jmap](https://github.com/pimalaya/io-jmap), [io-maildir](https://github.com/pimalaya/io-maildir), [io-smtp](https://github.com/pimalaya/io-smtp): per-protocol I/O-free coroutines and their std-blocking clients.
- [pimalaya/stream](https://github.com/pimalaya/stream): TCP / TLS / SASL plumbing shared by all std clients.
- [pimalaya/cli](https://github.com/pimalaya/cli): cross-binary CLI helpers (prompt, wizard primitives, clap args, build-time env, spinner).
- [pimalaya/config](https://github.com/pimalaya/config): TOML configuration loader and shell-expanded secrets.
- [pimalaya/mml](https://github.com/pimalaya/mml): MIME Meta Language used by the composer.
- [pimconf](https://github.com/pimalaya/pimconf): PIM service discovery (PACC, Thunderbird Autoconfiguration, RFC 6186 SRV) consumed by the wizard.

Bugs touching protocol semantics usually live in the matching `io-*` crate; rendering, key handling and the wizard flow live here.

## Override dependencies

`Cargo.toml` patches every Pimalaya crate at a local checkout next to this one:

```toml
[patch.crates-io]
io-email.path = "../io-email"
io-http.path = "../io-http"
io-imap.path = "../io-imap"
io-jmap.path = "../io-jmap"
io-maildir.path = "../io-maildir"
io-smtp.path = "../io-smtp"
mml.path = "../mml"
pimalaya-cli.path = "../cli"
pimalaya-config.path = "../config"
pimalaya-stream.path = "../stream"
pimconf.path = "../pimconf"
```

To build against the published `master` of each lib, swap the matching `.path = "../<repo>"` for `.git = "https://github.com/pimalaya/<repo>"` (the commented block at the bottom of `Cargo.toml` lists them all).

If cargo complains about *"perhaps two different versions of crate X are being used"*, patch every Pimalaya crate that pulls X transitively so the dep graph converges on the local copies.

## Contributing a theme preset

Presets live as plain Rust files under [`src/tui/theme/`](./src/tui/theme/) and are shipped with the binary. Adding one is three steps:

1. Create `src/tui/theme/<your_theme>.rs` exporting `pub const THEME: Theme = Theme { … };`. Copy [`src/tui/theme/dracula_dark.rs`](./src/tui/theme/dracula_dark.rs) as a starting template — every field is required, since the const is the source of truth for that preset.
2. Register the module in [`src/tui/theme/mod.rs`](./src/tui/theme/mod.rs): `pub mod your_theme;`.
3. Add a variant + match arm to `PresetConfig` in [`src/config.rs`](./src/config.rs): the variant name (in PascalCase) becomes the kebab-case `preset = "…"` value users put in their config.

Themable elements (with each one a `Style`) are listed on the [`Theme`](./src/tui/theme/theme.rs) struct. The built-in default uses named ANSI colors so the rendering blends with the user's terminal palette; bespoke presets typically use 24-bit RGB (`Color::Rgb(r, g, b)`) to match a fixed palette.

## Lint, test, audit

```
cargo fmt
cargo clippy --all-features --all-targets
cargo test --all-features
cargo deny check
```

## Commit style

Himalaya TUI follows the [conventional commits specification](https://www.conventionalcommits.org/en/v1.0.0/#summary). Prefix every commit with one of `feat`, `fix`, `refactor`, `docs`, `chore`, `test`, `ci`, `build`, optionally scoped (`fix(wizard): …`).
