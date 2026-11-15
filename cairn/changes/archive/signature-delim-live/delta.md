---
cairn: delta
change: signature-delim-live
---

# Delta

## ADDED Requirements

### Requirement: The signature is assembled from its two keys

The composer SHALL append `signature` introduced by `signature-delim`, resolved account-over-global. `signature` is the signature alone, `signature-delim` decides the separator, defaults to the RFC 3676 §4.3 `"-- \n"`, and is written verbatim so a delimiter without a trailing newline is expressible. The block SHALL be assembled exactly as the himalaya CLI assembles it, one configured value reading the same whichever binary composes.

#### Scenario: No signature is configured

Given an account and a global block declaring no `signature`, when a draft is composed, then nothing is appended, not even the delimiter.

#### Scenario: A delimiter of one's own

Given `signature = "Regards"` and `signature-delim = "~~~\n"`, when a draft is composed, then the body ends with the delimiter on its own line and the signature under it.
