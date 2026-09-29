---
cairn: log
change: announce-blocking-actions
landed: 2026-09-29
---

# Blocking actions are announced before they run

Issue #4. Every blocking action set its `…ing` status and ran in the same update, so the next draw came after the call returned and the status was never seen.

The domain messages (`ReadSelected`, `CopySelectedToTarget`, `SendCompose`, `LoadEnvelopes`, …) now go through `announce`, which sets the status and parks `Message::Run(Blocking)` in `Model::deferred`. The loop in src/tui/app.rs runs it right after the draw and loops back to draw again. `Blocking` carries only the envelope landing: the selection and the open dialog stay put for the one frame between, so the step functions read them as before, and the copy, move and flag dialogs now close in that second step.

`select_mailbox` no longer sets its own status, `announce` naming the mailbox for every envelope load, paging included. The "Compiling message…" status is gone: compiling is local and runs in the send step under `Sending message…`.

The `Run` step clears the status before acting, since reading, replying and forwarding set none on success and the announcing one would otherwise stay.

New capability: cairn/spec/interface.md.
