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

/// The Lua program that runs a bare `.glsl` file: full-screen shader with
/// the mouse feeding iMouse, Esc to stop. The shader source is embedded as
/// a long string so its line numbers are preserved in error messages.
pub fn glsl_wrapper(glsl: &str) -> String {
    // Pick a long-bracket level that does not occur in the source.
    let mut level = 0;
    while glsl.contains(&format!("]{}]", "=".repeat(level))) {
        level += 1;
    }
    let eq = "=".repeat(level);
    format!(
        "alt_screen()\nlocal sh = load_shader([{eq}[{glsl}]{eq}])\nwhile true do\n  draw_shader(sh)\n  flip()\nend\n"
    )
}
