//! Terminal UI built on the Elm Architecture (Model-Update-View).
//!
//! [`model`] owns every piece of state, including the
//! [`crate::shared::client::EmailClient`], and defines the
//! [`model::Message`] enum that names each transition. [`update`] is
//! the single transition function from `(Model, Message)` to a new
//! `Model` plus an optional follow-up `Message`; all I/O lives there
//! and nothing else mutates state. [`view`] renders the current model
//! to a ratatui [`Frame`] and never produces a message. [`app`] drives
//! the loop: poll events, fold the resulting message chain through
//! `update`, then redraw through `view`. [`theme`] holds the resolved
//! colors every render function reads.
//!
//! A single state container plus a single transition function makes the
//! data flow easy to follow, eliminates ad-hoc callbacks, and keeps
//! rendering side-effect-free. The pattern scales from a counter demo
//! to a multi-pane email client without changing shape. It is described
//! at <https://ratatui.rs/concepts/application-patterns/the-elm-architecture/>
//! and at <https://guide.elm-lang.org/architecture/>.
//!
//! [`Frame`]: ratatui::Frame

pub mod app;
pub mod model;
pub mod theme;
pub mod update;
pub mod view;
