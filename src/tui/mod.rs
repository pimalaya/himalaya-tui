//! Terminal UI built on the Elm Architecture (Model-Update-View).
//!
//! - [`model`] owns every piece of state, including the
//!   `EmailClientStd`, and defines the [`model::Message`] enum that
//!   names each transition.
//! - [`update`] is a pure function from `(Model, Message)` to a new
//!   `Model` plus an optional follow-up `Message`. All I/O lives
//!   here; nothing else mutates state except delegated sub-updates
//!   ([`palette::update`], reached through one `Message::Palette` arm).
//! - [`view`] renders the current `Model` to a ratatui [`Frame`]; it
//!   never produces messages or mutates anything user-visible.
//! - [`app`] drives the loop: poll events, fold the resulting message
//!   chain through `update`, then redraw via `view`.
//!
//! Why TEA: a single state container plus a single transition
//! function makes the data flow easy to follow, eliminates ad-hoc
//! callbacks, and keeps rendering side-effect-free. The pattern
//! scales from a counter demo to a multi-pane email client without
//! changing shape.
//!
//! References:
//! - <https://ratatui.rs/concepts/application-patterns/the-elm-architecture/>
//! - <https://guide.elm-lang.org/architecture/>
//!
//! [`Frame`]: ratatui::Frame

pub mod app;
pub mod command;
pub mod model;
pub mod palette;
pub mod theme;
pub mod update;
pub mod view;
