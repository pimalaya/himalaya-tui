---
cairn: change
id: signature-delim-live
status: landed
created: 2026-08-16
---

# Make `signature-delim` do something, and settle what `signature` holds

`signature-delim` has been in the configuration model and in config.sample.toml since the start, documented as the separator inserted between the body and the signature, and nothing has ever read it. The composer takes `signature` from the config and hands it straight to mml's template builders, which append what they are given verbatim, so the separator has to be written inside the value: `signature = "-- \nRegards"`.

The himalaya CLI just took the same two keys back (see its `compose-signature` change), where the opposite convention holds: `compose_body` writes the separator itself, so the value is the signature alone. One TOML file backs both binaries, so leaving this be would mean one value rendering two different bodies depending on which one composed it.

## What changes

`signature_block` in cli.rs assembles the separator and the signature before the model carries them, so `model.signature` is the finished block mml appends. `signature-delim` becomes the thing that decides the separator, defaulting to the RFC 3676 §4.3 `"-- \n"`, and is used verbatim so a delimiter without a trailing newline is expressible. An account declaring no signature yields an empty block, since a bare `-- ` on every draft is not what an unset key means.

This reinterprets `signature`: a value that carried `-- \n` now gets one prepended. That is a breaking change for anyone who wrote one, and the moment to take it is now, this binary being at v0.1.0 with its initial release unreleased. Detecting a delimiter already inside the value and stripping it would be worse than the breakage.

mml's template body already separates segments with a blank line, so the layout matches the CLI's without either side arranging it.
