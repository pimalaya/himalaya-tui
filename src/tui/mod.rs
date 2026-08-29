//! # TUI
//!
//! Terminal user interface built on the Elm Architecture: one state
//! container, one transition function, and rendering free of side
//! effects.
//!
//! [`model`] owns every piece of state and names each transition as a
//! [`model::Message`]. [`update`] is the single function taking
//! `(Model, Message)` to a new model plus an optional follow-up
//! message, and the only place where I/O happens.
//!
//! [`view`] renders the model to a ratatui [`Frame`] and never produces
//! a message, reading its colors from [`theme`]. [`app`] drives the
//! loop: poll events, fold the resulting message chain through update,
//! then redraw.
//!
//! The pattern is described at
//! <https://ratatui.rs/concepts/application-patterns/the-elm-architecture/>
//! and at <https://guide.elm-lang.org/architecture/>.
//!
//! [`Frame`]: ratatui::Frame

pub mod app;
pub mod model;
pub mod theme;
pub mod update;
pub mod view;
