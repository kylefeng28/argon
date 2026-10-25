//! The Argon front end: the shell, the editor and the console state machine
//! that owns them and the running program. Everything here draws through a
//! [`argon_core::TextGrid`] plus draw commands; no GPU code.

pub mod console;
pub mod lineedit;
pub mod shell;

pub use console::{Console, FrameOutput};
