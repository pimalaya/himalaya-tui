# Contributing guide

Thank you for investing your time in contributing to Himalaya TUI.

Whether you are a human or an AI agent, read these in order before touching the code:

1. the [Pimalaya README](https://github.com/pimalaya) for what the project is and how its repositories stack;
2. the [Pimalaya CONTRIBUTING](https://github.com/pimalaya/.github/blob/master/CONTRIBUTING.md) guide (Nix environment, build and check commands, dependency overrides, commit style), which chains to the shared architecture and guidelines;
3. the inline header documentation in src/main.rs: it is the architecture document of this binary, covering the three-pane TUI, the backends and plumbing, and the wizard flow;
4. the cairn/ folder for the development history and living plans (the Cairn convention: spec/, changes/, log/).

Everything below documents only what differs from the Pimalaya standards.

## Where changes belong

Himalaya TUI is a thin terminal front-end on top of the Pimalaya email stack, driving the sans-I/O io- libraries. The same backend code powers the [himalaya](https://github.com/pimalaya/himalaya) CLI. Triage before patching, since protocol and storage fixes usually belong upstream:

- IMAP, JMAP and SMTP wire semantics belong in [io-imap](https://github.com/pimalaya/io-imap), [io-jmap](https://github.com/pimalaya/io-jmap) and [io-smtp](https://github.com/pimalaya/io-smtp);
- local storage semantics belong in [io-maildir](https://github.com/pimalaya/io-maildir) and [io-m2dir](https://github.com/pimalaya/io-m2dir);
- account discovery consumed by the wizard belongs in [io-pim-discovery](https://github.com/pimalaya/io-pim-discovery);
- rendering, key handling, composition, the wizard and the shared cross-protocol surface live here.

The prompt, wizard and spinner primitives come from [pimalaya/cli](https://github.com/pimalaya/cli), the TOML loader and secret resolution from [pimalaya/config](https://github.com/pimalaya/config), the TCP, TLS and proxy plumbing from [pimalaya/stream](https://github.com/pimalaya/stream), the SASL mechanisms from [io-sasl](https://github.com/pimalaya/io-sasl), and MIME composition from [pimalaya/mml](https://github.com/pimalaya/mml). The src/main.rs header maps each backend to its crate.

## Feature matrix

Himalaya TUI is a binary, not a layered library, so it has no coroutine/client split. Its cargo features gate the backends (`imap`, `smtp`, `jmap`, `maildir`, `m2dir`) and the TLS provider (`rustls-ring` default, `rustls-aws`, `native-tls`), all on by default. A build needs at least one storage backend (`imap`, `jmap`, `maildir` or `m2dir`); `smtp` alone is a transport with nothing to read. Build a reduced set to check the feature gates still hold when touching them:

```sh
cargo build --no-default-features --features imap,smtp,rustls-ring
cargo build --no-default-features --features maildir,rustls-ring
```

## Dependencies

Every dependency resolves from crates.io except mime-meta-language, which the `[patch.crates-io]` table pins to its git repository until a release carries the composer's API; the pin comes out then. To build against a local checkout of a Pimalaya crate, add a `<crate>.path = "../<repo>"` entry there. If cargo reports two versions of a crate, patch every Pimalaya crate that pulls it transitively so the graph converges on the local copies.

## Contributing a theme preset

Presets live as plain Rust files under [src/tui/theme](./src/tui/theme/) and are shipped with the binary. Adding one is three steps:

1. Create src/tui/theme/&lt;your_theme&gt;.rs exporting `pub const THEME: Theme = Theme { … };`. Copy [src/tui/theme/dracula_dark.rs](./src/tui/theme/dracula_dark.rs) as a starting template: every field is required, since the const is the source of truth for that preset.
2. Declare the module in [src/tui/theme.rs](./src/tui/theme.rs): `pub mod your_theme;`.
3. Add a variant and match arm to `PresetConfig` in [src/config.rs](./src/config.rs): the variant name (in PascalCase) becomes the kebab-case `preset = "…"` value users put in their config.

Themable elements (each a `Style`) are listed on the `Theme` struct in [src/tui/theme.rs](./src/tui/theme.rs). The built-in default uses named ANSI colors so the rendering blends with the user's terminal palette; bespoke presets typically use 24-bit RGB to match a fixed palette.
