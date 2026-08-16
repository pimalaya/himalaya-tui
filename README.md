<div align="center">
  <img src="./logo.svg" alt="Logo" width="128" height="128" />
  <h1>📫 Himalaya TUI</h1>
  <p>TUI to manage emails</p>
  <p>
    <a href="https://matrix.to/#/#pimalaya:matrix.org"><img alt="Matrix" src="https://img.shields.io/badge/chat-%23pimalaya-blue?style=flat&logo=matrix&logoColor=white"/></a>
    <a href="https://fosstodon.org/@pimalaya"><img alt="Mastodon" src="https://img.shields.io/badge/news-%40pimalaya-blue?style=flat&logo=mastodon&logoColor=white"/></a>
    <a href="https://pimalaya.org/sponsor/"><img alt="Sponsor" src="https://img.shields.io/badge/sponsor-pink?style=flat&logo=github-sponsors&logoColor=white"/></a>
  </p>
</div>

![screenshot](./screenshot.jpeg)

> [!CAUTION]
> Himalaya TUI is in active development and currently shipped as `v0.x.x`. Expect breaking changes between releases until stabilization.

## Table of contents

- [Features](#features)
- [Installation](#installation)
  - [Pre-built binary](#pre-built-binary)
  - [Cargo](#cargo)
  - [Nix](#nix)
  - [Sources](#sources)
- [Configuration](#configuration)
  - [Starting without a configuration](#starting-without-a-configuration)
  - [Provider recipes](#provider-recipes)
  - [Theming](#theming)
- [Usage](#usage)
  - [Keybindings](#keybindings)
  - [Composing messages](#composing-messages)
  - [Re-using sessions](#re-using-sessions)
- [Interfaces](#interfaces)
- [AI policy](https://github.com/pimalaya/.github/blob/master/AI_POLICY.md)
- [License](#license)
- [Social](#social)
- [Contributing](./CONTRIBUTING.md)
- [Sponsoring](#sponsoring)

## Features

- Remote backend support: **IMAP**, **SMTP**, **JMAP**
- Local (filesystem) backends support: **Maildir** <sup>[specs](https://cr.yp.to/proto/maildir.html)</sup>, **m2dir** <sup>[specs](https://man.sr.ht/~bitfehler/m2dir/)</sup>
- **Simple auth** support for IMAP/SMTP: anonymous, login, plain, oauthbearer, xoauth2, scram-sha-256
- **HTTP auth** support for JMAP: basic, bearer
- **TLS** support:
  - [Rustls](https://crates.io/crates/rustls) with ring crypto (requires `rustls-ring` feature, enabled by default)
  - [Rustls](https://crates.io/crates/rustls) with aws crypto (requires `rustls-aws` feature)
  - [Native TLS](https://crates.io/crates/native-tls) (requires `native-tls` feature)
- **Discovery** support:
  - PACC <sup>[specs](https://datatracker.ietf.org/doc/html/draft-ietf-mailmaint-pacc)</sup>
  - Autoconfiguration (Thunderbird) <sup>[specs](https://wiki.mozilla.org/Thunderbird:Autoconfiguration)</sup>
  - SRV DNS lookups <sup>[rfc6186](https://datatracker.ietf.org/doc/html/rfc6186)</sup>
- **SOCKS5**, **HTTP** proxy support via `all_proxy`, `https_proxy` and `no_proxy`
- **Three-pane layout** built on [ratatui](https://ratatui.rs): mailboxes, envelopes, message body or composer
- **In-app composer** powered by [edtui](https://crates.io/crates/edtui) with system-editor handoff (`Alt-e`)
- **Color themes**: built-in presets plus per-field overrides in the config (see [Theming](#theming))
- **Shared configuration file** with `himalaya`: same `[accounts.<name>]` blocks load on both binaries (see [Configuration](#configuration))

> [!TIP]
> Himalaya TUI is written in [Rust](https://www.rust-lang.org/) and uses [cargo features](https://doc.rust-lang.org/cargo/reference/features.html) to gate backend support. The default feature set is declared in [Cargo.toml](./Cargo.toml).

## Installation

### Pre-built binary

Himalaya TUI is not yet released, therefore the only way to get a pre-built binary is to check out the [releases](https://github.com/pimalaya/himalaya-tui/actions/workflows/releases.yml) GitHub workflow and look for the *Artifacts* section.

> [!NOTE]
> Such binaries are built with the default cargo features. If you need specific features, please use another installation method.

### Cargo

```sh
cargo install --locked --git https://github.com/pimalaya/himalaya-tui.git
```

With only IMAP+SMTP support:

```sh
cargo install --locked --git https://github.com/pimalaya/himalaya-tui.git \
  --no-default-features \
  --features imap,smtp,rustls-ring
```

### Nix

If you have the [Flakes](https://nixos.wiki/wiki/Flakes) feature enabled:

```sh
nix profile install github:pimalaya/himalaya-tui
```

Or run without installing:

```sh
nix run github:pimalaya/himalaya-tui
```

### Sources

```sh
git clone https://github.com/pimalaya/himalaya-tui
cd himalaya-tui
nix run
```

## Configuration

Himalaya TUI reads a configuration, it never writes one. Accounts are authored by the [himalaya](https://github.com/pimalaya/himalaya) CLI, whose wizard proposes an `[accounts.<name>]` table for your file, or by hand against [config.sample.toml](./config.sample.toml). There is no `configure` command here.

A configuration is loaded from the first valid path among:

- $XDG_CONFIG_HOME/himalaya/config.toml
- $HOME/.config/himalaya/config.toml
- $HOME/.himalayarc

These are the same paths the [himalaya](https://github.com/pimalaya/himalaya) CLI looks at: one TOML file backs both binaries, **starting from himalaya CLI v2**. TUI-only fields and CLI-only sections coexist without errors. See [config.sample.toml](./config.sample.toml) for a documented template.

> [!WARNING]
> A himalaya CLI v1 configuration file is **not** compatible with himalaya TUI: the v1 schema differs from the v2 one shared with the TUI. Upgrade the CLI to v2 (or rewrite the file using [config.sample.toml](./config.sample.toml)) before pointing the TUI at it.

Override the path with `-c <PATH>` or `HIMALAYA_CONFIG=<PATH>`; multiple paths can be passed at once, separated by `:`. The first one is the base and the rest are deep-merged on top.

Pick the account to open with `-a <NAME>`, or let the one flagged `default = true` be used. An unknown name is an error listing the accounts your file does hold.

### Starting without a configuration

A run that resolves no account still has to open something, so it falls back to a wizard that builds one **in memory, for that session only**. Nothing reaches your file, and nothing is proposed for it: to keep an account, author it with the CLI or by hand.

You can ask for that fallback outright, in two ways. The positional argument seeds it, so `himalaya-tui you@fastmail.com` skips the account lookup and opens a throwaway account discovered from that address, keeping the rest of your file (theme, signature, keybindings). `--no-config` drops the file whole, theme included, and prompts for the address instead. Both are silent, having been asked for, and neither can be combined with `-a`.

The fallback also happens by accident, and there it warns on stderr naming what was missing, since it usually means a mistyped path or a forgotten `default = true`:

```
✗ No configuration file at /home/you/.config/himalaya/config.toml, falling back to an in-memory account
✗ Configuration file carries no default account, falling back to an in-memory account
```

Either way the wizard asks for an email address unless the positional argument already supplied one, probes the discovery mechanisms in series until one answers, then asks for the SASL or HTTP credentials that mechanism needs. A server URL (`imaps://mail.example.com`) and a local folder path (`file:///home/you/Mail`) are accepted wherever the address is, they are just not what the prompt asks for. DNS goes through the host's own resolver, falling back to Cloudflare's `1.1.1.1` over TCP when it finds none; override it with `HIMALAYA_DNS_RESOLVER=<URL>`.

### Provider recipes

The account blocks are the ones the CLI documents, so the ready-made configurations for Proton Mail, Fastmail, Gmail, Outlook, Posteo and iCloud Mail live once, in the [himalaya Configuration section](https://github.com/pimalaya/himalaya#configuration), and apply verbatim here. Only the CLI-only keys around them are ignored by the TUI.

### Theming

The TUI uses named ANSI colors by default, so the rendering inherits the colors of your terminal palette. Pick a preset and/or override individual fields in the `[theme]` block of your config (full reference in [config.sample.toml](./config.sample.toml)):

```toml
[theme]
preset = "dracula-dark"

[theme.cursor]
fg = "magenta"
bg = "#222"
mod = ["bold", "italic"]
```

Color values accept named ANSI (`"blue"`, `"dark-gray"`, …), hex (`"#ff8800"`), 256-color indices (`"33"`), or `"reset"` for the terminal default. `mod` is a list of `bold`, `dim`, `italic`, `underlined`, `slow-blink`, `rapid-blink`, `reversed`, `hidden`, `crossed-out`.

Overrides are merged on top of the preset: any field you leave out keeps the preset value, so you can change just one attribute (e.g. only the cursor `fg`) and inherit the rest. Themable elements: `header`, `status-bar`, `border-active`, `border-inactive`, `dialog-border`, `cursor`, `mailbox-current`, `envelope-header`, `envelope-seen`, `envelope-unread`, `message-body`, `compose-text`, `compose-cursor`, `compose-selection`.

The presets shipped with the binary are `default` (named ANSI, the built-in), `dracula-dark`, `one-light` and `tokyo-night`. They live as plain Rust files under [src/tui/theme](./src/tui/theme/); pull requests adding new presets are welcome (see [CONTRIBUTING.md](./CONTRIBUTING.md)).

## Usage

### Keybindings

Top-level navigation, supporting Vim and Emacs keybinds:

| Keybind | Action |
|---|---|
| `Tab` | Cycle panel |
| `↓`, `j`, `Ctrl-n` | Next item |
| `↑`, `k`, `Ctrl-p` | Previous item |
| `PageDown`, `Ctrl-d`, `Ctrl-v` | Next page |
| `PageUp`, `Ctrl-u`, `Alt-v` | Previous page |
| `Enter` | Select |
| `Esc`, `q`, `Ctrl-g` | Close panel / dialog / quit |
| `Ctrl-c` | Start a new draft |

Composer:

| Key | Action |
|---|---|
| `Ctrl-e`, `Alt-e` | Hand off to `$VISUAL` or `$EDITOR` for the current draft |
| `Esc` | Open the compose actions dialog (Send, Preview, Save to Drafts, Cancel) |

The `--keybinds <vim|emacs>` flag (and the top-level `keybinds = "emacs"` TOML field) changes the in-app composer's edtui keybinds. In Vim mode, `Ctrl-e` (edtui's normal-mode binding) opens the external editor; in Emacs mode, `Ctrl-e` is rebound to "move to end of line" and `Alt-e` is the only system-editor key.

Envelope dialog actions: Read, Reply, Reply All, Forward, Copy, Move, Add flag, Remove flag.

### Composing messages

Drafts are written in [MML](https://github.com/pimalaya/mml) and compiled to MIME on send. Headers (`From`, `To`, `Subject`…) live at the top of the buffer; the body and any MML directives (attachments, signing, encryption) follow.

Sending routes through the storage backend when it can send on its own (JMAP), otherwise through the `[accounts.<name>.smtp]` transport, which connects on the first send rather than at startup. Drafts can be saved to the `Drafts` mailbox at any time.

### Re-using sessions

An `imap.server` or `smtp.server` given as `unix:///path/to/socket` reaches a local socket proxy such as [sirup](https://github.com/pimalaya/sirup), whose greeting is already authenticated: no credentials are configured on this side and none are negotiated over the socket.

## Interfaces

Himalaya TUI is one of several front-ends to the Pimalaya libraries. See [pimalaya/himalaya#interfaces](https://github.com/pimalaya/himalaya#interfaces) for the full list (CLI, Vim, Emacs, Raycast).

## License

This project is licensed under either of:

- [MIT license](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

## Social

- Chat on [Matrix](https://matrix.to/#/#pimalaya:matrix.org)
- News on [Mastodon](https://fosstodon.org/@pimalaya) or [RSS](https://fosstodon.org/@pimalaya.rss)
- Mail at [pimalaya.org@posteo.net](mailto:pimalaya.org@posteo.net)

## Sponsoring

[![nlnet](https://nlnet.nl/logo/banner-160x60.png)](https://nlnet.nl/)

Special thanks to the [NLnet foundation](https://nlnet.nl/) and the [European Commission](https://www.ngi.eu/) that have been financially supporting the project for years:

- 2022 → 2023: [NGI Assure](https://nlnet.nl/project/Himalaya/)
- 2023 → 2024: [NGI Zero Entrust](https://nlnet.nl/project/Pimalaya/)
- 2024 → 2026: [NGI Zero Core](https://nlnet.nl/project/Pimalaya-PIM/)
- 2026 → 2027: [NGI Zero Commons Fund](https://nlnet.nl/project/Pimalaya-pimdir/)

This program is part of Pimalaya, free software funded entirely by grants and donations. If you find it useful, consider [sponsoring](https://pimalaya.org/sponsor/) its development:

[![GitHub](https://img.shields.io/badge/-GitHub%20Sponsors-fafbfc?logo=GitHub%20Sponsors)](https://github.com/sponsors/soywod)
[![Ko-fi](https://img.shields.io/badge/-Ko--fi-ff5e5a?logo=Ko-fi&logoColor=ffffff)](https://ko-fi.com/soywod)
[![Buy Me a Coffee](https://img.shields.io/badge/-Buy%20Me%20a%20Coffee-ffdd00?logo=Buy%20Me%20A%20Coffee&logoColor=000000)](https://www.buymeacoffee.com/soywod)
[![Liberapay](https://img.shields.io/badge/-Liberapay-f6c915?logo=Liberapay&logoColor=222222)](https://liberapay.com/soywod)
[![thanks.dev](https://img.shields.io/badge/-thanks.dev-000000?logo=data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjQuMDk3IiBoZWlnaHQ9IjE3LjU5NyIgY2xhc3M9InctMzYgbWwtMiBsZzpteC0wIHByaW50Om14LTAgcHJpbnQ6aW52ZXJ0IiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxwYXRoIGQ9Ik05Ljc4MyAxNy41OTdINy4zOThjLTEuMTY4IDAtMi4wOTItLjI5Ny0yLjc3My0uODktLjY4LS41OTMtMS4wMi0xLjQ2Mi0xLjAyLTIuNjA2di0xLjM0NmMwLTEuMDE4LS4yMjctMS43NS0uNjc4LTIuMTk1LS40NTItLjQ0Ni0xLjIzMi0uNjY5LTIuMzQtLjY2OUgwVjcuNzA1aC41ODdjMS4xMDggMCAxLjg4OC0uMjIyIDIuMzQtLjY2OC40NTEtLjQ0Ni42NzctMS4xNzcuNjc3LTIuMTk1VjMuNDk2YzAtMS4xNDQuMzQtMi4wMTMgMS4wMjEtMi42MDZDNS4zMDUuMjk3IDYuMjMgMCA3LjM5OCAwaDIuMzg1djEuOTg3aC0uOTg1Yy0uMzYxIDAtLjY4OC4wMjctLjk4LjA4MmExLjcxOSAxLjcxOSAwIDAgMC0uNzM2LjMwN2MtLjIwNS4xNTYtLjM1OC4zODQtLjQ2LjY4Mi0uMTAzLjI5OC0uMTU0LjY4Mi0uMTU0IDEuMTUxVjUuMjNjMCAuODY3LS4yNDkgMS41ODYtLjc0NSAyLjE1NS0uNDk3LjU2OS0xLjE1OCAxLjAwNC0xLjk4MyAxLjMwNXYuMjE3Yy44MjUuMyAxLjQ4Ni43MzYgMS45ODMgMS4zMDUuNDk2LjU3Ljc0NSAxLjI4Ny43NDUgMi4xNTR2MS4wMjFjMCAuNDcuMDUxLjg1NC4xNTMgMS4xNTIuMTAzLjI5OC4yNTYuNTI1LjQ2MS42ODIuMTkzLjE1Ny40MzcuMjYuNzMyLjMxMi4yOTUuMDUuNjIzLjA3Ni45ODQuMDc2aC45ODVabTE0LjMxNC03LjcwNmgtLjU4OGMtMS4xMDggMC0xLjg4OC4yMjMtMi4zNC42NjktLjQ1LjQ0NS0uNjc3IDEuMTc3LS42NzcgMi4xOTVWMTQuMWMwIDEuMTQ0LS4zNCAyLjAxMy0xLjAyIDIuNjA2LS42OC41OTMtMS42MDUuODktMi43NzQuODloLTIuMzg0di0xLjk4OGguOTg0Yy4zNjIgMCAuNjg4LS4wMjcuOTgtLjA4LjI5Mi0uMDU1LjUzOC0uMTU3LjczNy0uMzA4LjIwNC0uMTU3LjM1OC0uMzg0LjQ2LS42ODIuMTAzLS4yOTguMTU0LS42ODIuMTU0LTEuMTUydi0xLjAyYzAtLjg2OC4yNDgtMS41ODYuNzQ1LTIuMTU1LjQ5Ny0uNTcgMS4xNTgtMS4wMDQgMS45ODMtMS4zMDV2LS4yMTdjLS44MjUtLjMwMS0xLjQ4Ni0uNzM2LTEuOTgzLTEuMzA1LS40OTctLjU3LS43NDUtMS4yODgtLjc0NS0yLjE1NXYtMS4wMmMwLS40Ny0uMDUxLS44NTQtLjE1NC0xLjE1Mi0uMTAyLS4yOTgtLjI1Ni0uNTI2LS40Ni0uNjgyYTEuNzE5IDEuNzE5IDAgMCAwLS43MzctLjMwNyA1LjM5NSA1LjM5NSAwIDAgMC0uOTgtLjA4MmgtLjk4NFYwaDIuMzg0YzEuMTY5IDAgMi4wOTMuMjk3IDIuNzc0Ljg5LjY4LjU5MyAxLjAyIDEuNDYyIDEuMDIgMi42MDZ2MS4zNDZjMCAxLjAxOC4yMjYgMS43NS42NzggMi4xOTUuNDUxLjQ0NiAxLjIzMS42NjggMi4zNC42NjhoLjU4N3oiIGZpbGw9IiNmZmYiLz48L3N2Zz4=)](https://thanks.dev/soywod)
[![PayPal](https://img.shields.io/badge/-PayPal-0079c1?logo=PayPal&logoColor=ffffff)](https://www.paypal.com/paypalme/soywod)
