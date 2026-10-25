//! The Luau runtime for Argon programs: sandbox, API bindings, the frame
//! protocol, error normalization and the program thread.

pub mod api;
pub mod docs;
pub mod error;
pub mod machine;
pub mod program;
pub mod runtime;

pub use error::ArgonError;
pub use machine::{Flip, FrameAck, FrameOut, Machine, MachineOptions};
pub use program::{spawn_program, ProgramExit, ProgramHandle, ProgramSpec};
pub use runtime::Runtime;
