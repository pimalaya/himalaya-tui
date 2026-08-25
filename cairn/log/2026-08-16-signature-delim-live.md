---
cairn: log
change: signature-delim-live
landed: 2026-08-16
---

# `signature-delim` stopped being decoration

The key had been in the model and in config.sample.toml from the start, documented as the separator between the body and the signature, and nothing read it. The composer took `signature` from the config and handed it to mml, whose template builders append what they are given verbatim, so the separator had to be written inside the value.

The himalaya CLI took the same two keys back today under the opposite convention, its `compose_body` writing the separator itself. One file backs both binaries, so the value had to mean one thing.

## What landed

`signature_block` in cli.rs assembles the separator and the signature, and `model.signature` now carries the finished block. `signature-delim` decides the separator, defaults to the RFC 3676 §4.3 `"-- \n"`, and is used verbatim, so a delimiter without a trailing newline is expressible at all. An account declaring no signature yields an empty block: a bare `-- ` on every draft is not what an unset key means, which is the case the test pins.

mml's template body already separates segments with a blank line, so the draft lays out exactly like the CLI's message without either side arranging it.

**This reinterprets `signature`.** A value that carried `-- \n` now gets one prepended. Deliberate, and taken now: this binary is at v0.1.0 with its initial release unreleased, so there is no installed base, and detecting a delimiter inside the value to strip it would be worse than the breakage.

## Capabilities moved

- **configuration**: added the requirement that the signature is assembled from its two keys, the same way the himalaya CLI assembles it, with the no-signature and custom-delimiter scenarios.
