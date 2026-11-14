---
cairn: delta
change: onboarding-on-the-android-flow
---

## ADDED Requirements

### Requirement: A run with no account onboards one

With no account to open, the TUI SHALL run the onboarding in its own screens: an address, discovery, a pick, a review, then a password stored in the system keyring or an OAuth 2.0 hand-off, a server URL asked only when nothing was found. It SHALL write the account to the configuration file only once himalaya's `account check` passed on it, and SHALL open the session on it.

#### Scenario: A first run

Given no configuration file, when the TUI starts and the onboarding completes, then the file holds the new account flagged default, no password is written to it, and the interface opens on the inbox.

## MODIFIED Requirements

## REMOVED Requirements

### Requirement: No configuration stops before the screen

Replaced by "A run with no account onboards one": with no account to open, the TUI onboards one rather than stopping.
