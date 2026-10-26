# Argon - High-Level Technical Design

Companion to `requirements.md`; requirement IDs (FR-*, NFR-*) are referenced throughout.

## 1. Goals and constraints that shape the design

- Linear program model (D7): the user's Lua runs top to bottom and blocks in
  `flip()`. The host cannot "call into" the program each frame; the program
  calls the host. This is the single most important architectural driver.
- Esc must break any program within 100 ms, including one that never yields
  (FR-R6, NFR-7).
- Real GPU, Shadertoy GLSL (D6), render-to-texture, 100k sprites/frame
  (FR-G9).
- Keystroke latency in shell and editor within one frame (NFR-2).
- Sandbox enforced in the runtime (NFR-6).
- Must not preclude a browser build (D2, NFR-5), which forbids designs that
  require blocking the main thread or spawning OS threads without a fallback.
- Headless rendering for CI and for developer machines without a display
  (FR-T1, NFR-13).

## 2. Architecture overview

```
+---------------------------------------------------------------------+
|  argon (binary)                                                     |
|  winit event loop  |  CLI (run/check/docs)  |  config, themes       |
+----------------------------+----------------------------------------+
|  argon-shell               |  argon-lua                             |
|  Shell app, Editor app,    |  Program thread, API bindings,         |
|  Catalog, Help, immediate  |  sandbox, instruction hook, errors     |
|  mode (uses argon-lua)     |                                        |
+----------------------------+----------------------------------------+
|  argon-core                                                         |
|  DrawCmd / FrameCommands, Resource handles, InputSnapshot,          |
|  TextGrid, Palette, Cart manifest, VFS, Api table (docs source)     |
+------------------+-------------------------+------------------------+
|  argon-gfx       |  argon-audio            |  argon-vfs (in core)   |
|  wgpu renderer:  |  cpal output, mixer,    |  host dir mapping,     |
|  framebuffer,    |  decoders (hound,       |  path jail, carts,     |
|  batch, text,    |  lewton), synth voices  |  import/export (zip)   |
|  GLSL shaders    |                         |                        |
|  via naga,       |                         |                        |
|  presenter       |                         |                        |
+------------------+-------------------------+------------------------+
```

Cargo workspace with one crate per box. `argon-core` has no GPU, audio or
Lua dependencies so it compiles everywhere (including wasm) and is the place
shared types live.

### 2.1 Threads and ownership

Three threads on desktop:

1. Main thread. Owns the winit window, the wgpu device and surface, the
   renderer, the audio handle, and the "front" state machine (Shell, Editor,
   Running). Receives OS events. Never blocks on the program.
2. Program thread. Owns the Lua state and runs the user's script. Blocks
   inside `flip()` waiting for the main thread to acknowledge the frame.
   Exists only while a program is running; torn down on exit or break.
3. Audio thread. cpal's callback thread. Owns the mixer. Receives commands
   through a lock-free queue; never allocates in the callback.

Communication is message passing over bounded channels. The program thread
never touches wgpu; it emits commands. This keeps the GPU single-threaded,
makes Esc trivially safe (the main thread just stops reading from a dead
program), and maps onto the browser later (section 11).

```
Program thread                        Main thread
--------------                        -----------
API call -> push DrawCmd              winit event -> InputState
...                                   
flip():                               loop {
  send FrameCommands  ------------->    recv FrameCommands (non-blocking)
  block on FrameAck   <-------------    render to framebuffer, present
                                        send FrameAck { InputSnapshot, dt }
                                      }
```

Resource creation (`load_image`, `load_shader`, `new_image`, `load_sound`)
is synchronous request/reply over a second channel because it returns handles
and, for shaders, compile errors must surface at the calling line (FR-R8).
These calls are rare relative to draws, so the round trip is acceptable.

### 2.2 Front state machine

```
Boot -> Shell <-> Editor
          |          |
          v          v
        Running (from shell)   Running (from editor)
          |                       |
          +--> Shell              +--> Editor (cursor on error line)
```

`Running` holds a `ProgramHandle` (join handle, command receivers, break flag,
resource table). Leaving `Running` drops the handle: the break flag is set, the
Lua hook raises, the thread joins with a timeout, GPU resources owned by the
program are released, audio voices are stopped (FR-A3).

## 3. Rendering (argon-gfx)

### 3.1 Targets and layers

- Framebuffer: an `Rgba8Unorm` texture at the cart's virtual resolution.
  Persistent across frames (FR-G8). All Tier 1 draws go here (or to a user
  image when `set_target` is active).
- Text layer: a grid of cells (cols x rows = resolution / 8x12) stored as a
  CPU buffer of `(glyph, fg, bg, flags)`; uploaded as a storage buffer and
  rendered by one instanced draw over the framebuffer. Shell and editor are
  pure text-layer users; programs mix both layers freely (FR-C6).
- Presenter: a final pass that samples the framebuffer (with text layer
  composited) onto the window surface with integer scale, letterbox, and an
  optional filter shader (FR-C3, FR-C4).

Resolution changes (`set_resolution` or cart manifest) recreate the
framebuffer and resize the text grid.

### 3.2 Batch renderer

All Tier 1 primitives and sprites are expressed as textured quads and
triangles in a single vertex format `(pos: vec2, uv: vec2, color: rgba8)`
with a per-draw texture index. Commands are sorted only by state change
(texture atlas page, blend mode, active shader, clip rect, target), preserving
submission order within a state. Circles, ellipses and polygons are
tessellated on the CPU into triangle fans (with a signed-distance fragment
path considered later for smoother large circles). `pixel` becomes a 1x1 quad;
runs of `pixel` calls in one frame coalesce into one draw.

Images live in texture arrays bucketed by size class to minimize binds. With a
single persistent vertex buffer (grown as needed) and instancing for sprites,
100k sprites is roughly 400k vertices, well within budget on integrated GPUs
(FR-G9).

### 3.3 Shaders (FR-G6, FR-G7, D6)

User shaders are GLSL fragment shaders. Pipeline:

1. The source is wrapped if it defines `mainImage(out vec4, in vec2)` and no
   `main()`: Argon prepends `#version 450`-style preamble, the Shadertoy
   uniform block (`iResolution`, `iTime`, `iTimeDelta`, `iFrame`, `iMouse`,
   `iDate` zeroed, `iChannel0..3` samplers + `iChannelResolution`), and
   appends a `main()` that calls `mainImage`.
2. naga's GLSL frontend parses it to naga IR; validation errors are mapped
   back to user line numbers (subtracting the preamble length) and raised as
   Lua errors in the standard format.
3. naga IR is handed to wgpu as a `ShaderModuleDescriptor` (wgpu accepts naga
   modules directly, no text round trip). A pipeline is created per
   (shader, blend, target format) and cached.

Two usage modes:

- `draw_shader(sh, opts)`: full-screen (or rect) quad, uniforms from
  Argon-provided values plus a user table mapped onto a uniform buffer by
  reflection of the naga module.
- `set_shader(sh)`: subsequent batch draws use the user fragment shader with
  Argon's standard vertex shader. The user shader receives `iChannel0` as the
  sprite texture and the vertex color as an input.

Shader compile time is hidden from the frame: `load_shader` is synchronous and
users are told to call it before the loop, as with Shadertoy.

Risks: naga's GLSL frontend lags behind its WGSL frontend; some Shadertoy
constructs (e.g. `texture` overloads, uninitialized out params, implicit
casts) may fail. Mitigation: a Phase 0 spike that runs the top 20 Shadertoy
shaders through naga, and a documented compatibility list.

### 3.4 Readback (FR-G10)

`get_pixel` and `save_image` copy the target to a staging buffer and map it.
To avoid one readback per call, the first `get_pixel` in a frame snapshots the
whole target into a CPU cache that is invalidated by any subsequent draw. This
keeps "read a few pixels after drawing" cheap and "interleave draw and read
per pixel" correct but slow, which the docs state.

In palette mode (FR-C5) the framebuffer is still RGBA but draws resolve palette
indices on the CPU; `get_pixel` returns the nearest palette index.

## 4. Text layer and fonts

- Built-in 8x12 bitmap font covering ASCII, Latin-1, box drawing and a block
  of console-style glyphs; packed into a single texture at build time.
- The grid is the unit of the shell and editor. Both render by writing cells;
  there is no other UI toolkit (D5, principle "the console is the UI").
- Programs use the grid through `print`/`locate`/`put_char`, and pixel text
  through `draw_text` which renders the same font onto the framebuffer.
- A theme is a palette of 16 colors plus default fg/bg (FR-C9).

## 5. Lua runtime (argon-lua)

### 5.1 Engine

`mlua` with the `luau` and `luau-jit` features. Luau gives: a fast interpreter (2 to 3x PUC Lua),
gradual typing (optional for users, useful for our own code), a sandbox
designed for untrusted scripts (no `load` of bytecode, read-only globals,
built-in interrupt callback), and good error messages. Fallback: `lua54`
feature if a Luau limitation bites; the API layer does not depend on which.

Native code generation is enabled for every chunk (`enable_jit`) together
with compiler optimization level 2; a Mandelbrot-style inner loop runs about
2x faster than the interpreter. Set `ARGON_NO_JIT=1` to disable it for
comparison. Interrupts (Esc) still fire inside JIT-compiled loops.

### 5.2 Sandbox (FR-R1, FR-R2, NFR-6)

- `Lua::new_with(StdLib::BASE | STRING | TABLE | MATH | UTF8 | COROUTINE,
  LuaOptions::new().catch_rust_panics(true))`.
- Remove `loadstring`/`load` with binary chunks (Luau already disallows
  bytecode), `dofile`, `loadfile`, `collectgarbage("count")` is allowed,
  `collectgarbage` other modes are no-ops.
- `require` is replaced by an Argon function that resolves relative module
  paths inside the cart folder through the VFS jail, caches modules, and
  rejects anything else.
- `Lua::set_memory_limit(bytes)` enforces FR-R7; the error is rewritten to
  "Program exceeded memory limit (512 MB)".
- Globals table is frozen (Luau `setreadonly`) after API installation so a
  program cannot monkey-patch `flip` to escape the hook.

### 5.3 Break and watchdog (FR-R6)

- A shared `AtomicBool` break flag. The main thread sets it on Esc (or on
  window close, or on headless frame budget exhaustion).
- Luau: `set_interrupt` callback is invoked periodically by the VM; it checks
  the flag and returns `VmState::Yield`-equivalent error "Break". For Lua 5.4:
  `set_hook(HookTriggers::every_nth_instruction(50_000))`.
- Native Argon functions that block (`flip`, `input`, `wait_key`, `wait`,
  resource creation) wait with a timeout loop on the channel and check the
  flag on every wake, so a program blocked in `flip()` also breaks.
- The error "Break" is uncatchable by user `pcall`: Argon wraps `pcall` and
  `xpcall` to rethrow it.
- If the thread does not join within 500 ms after the flag (a tight loop in a
  native call we did not expect), the main thread detaches it, leaks the Lua
  state deliberately, logs a warning, and proceeds. The user never waits.

### 5.4 Frame protocol

`flip()`:

1. Takes the accumulated `Vec<DrawCmd>` and sends `FrameCommands { cmds,
   text_grid_dirty_rows, audio_cmds }`.
2. Blocks on `FrameAck { input: InputSnapshot, dt, frame_no }` with the
   break-flag check loop.
3. Installs the snapshot as the program's current input state (FR-I4).
4. Returns.

Frame pacing (FR-R5) lives on the main thread: it presents at vsync and sends
the ack when the cart's fps interval has elapsed; so a 30 fps cart gets acks
every other vsync. If the program is late, the main thread still renders the
last frame and keeps the UI responsive; input events accumulate into the next
snapshot (pressed/released edges are not lost).

Draws between flips are recorded on the program thread as plain data
(`DrawCmd` enum, `Copy`-friendly), never GPU objects, so recording is cheap
and the renderer can be swapped (desktop wgpu vs future web).

### 5.5 Error presentation (FR-R8)

All Lua errors are normalized into `ArgonError { file, line, message, hint, source_line }`.

Hints:

- Unknown global: Levenshtein distance <= 2 against the API table and the
  program's own globals -> "Did you mean fill_rect?".
- Wrong argument type: the API table includes parameter names and types, so
  the message reads "fill_rect: argument 3 (w) expected number, got string".
- Shader errors: `shader "glow" line 14: ...` with the user's line number.

The same struct is rendered by the shell (text) and the editor (cursor jump +
status line).

### 5.6 API table as the single source of truth (NFR-9, FR-E3, FR-T3)

Every API function is declared once in Rust via a small macro:

```rust
api! {
    /// Fill a circle centered at (x, y).
    /// example: fill_circle(64, 64, 20, rgb(255, 80, 0))
    category: Draw,
    fn fill_circle(x: f32, y: f32, r: f32, color: Option<Color>) -> ();
}
```

The macro generates the mlua binding with argument checking and typed error
messages, an entry in `ApiTable` (name, category, params, doc, example), and
is what the editor Catalog, F-key menus, `HELP`, tab completion, "did you
mean" and `argon docs` read from. A unit test asserts that every function
registered into the Lua globals has an `ApiTable` entry with a non-empty
example that parses.

## 6. Audio (argon-audio)

- Output through `cpal` at the device's native rate, stereo f32.
- Mixer with 16 sample voices and 8 synth channels (FR-A1, FR-A2). Resampling
  by linear interpolation; pitch is a playback-rate multiplier.
- Decoders: `hound` (WAV), `lewton` (OGG Vorbis). Decoding happens on the
  program thread at `load_sound` time into an `Arc<[f32]>`; the audio thread
  only reads.
- Synth voices: square (with duty), triangle, saw, sine, noise (LFSR), with
  an ADSR envelope and linear frequency slide. Enough for "tiny synth" and for
  `play_notes` later.
- Commands (`Play`, `Stop`, `SetVolume`, `Tone`) flow through an `rtrb`
  ring buffer. On program exit the main thread sends `StopAll`.
- Headless mode uses a null backend so CI needs no sound device.

## 7. Virtual disk and carts (argon-core::vfs)

### 7.1 Layout

A program is either a single `name.lua` file or a cart folder containing `cart.toml` and `main.lua`. (FR-F3)

```
~/.argon/
  disk/            the virtual disk root (ARGON> is here)
    DEMOS/         read-only, copied from the binary on first boot/upgrade
    SAVE/          per-cart key-value stores, <cart>.json
    pong.lua       single-file program
    GAMES/
      shooter/     cart folder
        cart.toml
        main.lua
        enemies.lua
        sprites.png
        hit.wav
  export/          EXPORT output (.argon zips)
  config.toml      theme, key map, window size, disk path override
  history.txt      shell history
```

Portable mode (`--portable` or a `portable.flag` next to the exe) uses
`./argon-data/` instead of `~/.argon`.

### 7.2 Path jail (FR-F2)

`Vfs::resolve(cart_root, user_path) -> Result<HostPath>`:

- Normalize separators to `/`; reject empty, absolute, `..`, NUL, and reserved
  names; canonicalize the host path and assert it has the cart root or
  `SAVE/<cart>/` as a prefix after resolving symlinks. Symlinks that escape
  are rejected.
- The shell uses a wider jail: the whole `disk/`. The same function with a
  different root.
- All file I/O for programs goes through `Vfs`; no other code path opens host
  files on behalf of a program.

### 7.3 Manifest

The manifest for a cart looks like this:

```toml
# cart.toml
title = "Snake"
author = "Kyle"
api = 1
resolution = "classic"   # or "320x180", "native"
color = "truecolor"      # or palette = 16, palette_file = "pal.hex"
fps = 60
filter = "none"
```

Single-file `.lua` programs do not have a manifest file and instead use default values;
a `--- argon:` header comment block can be used to override `resolution`, `color`, `fps` (FR-F3).

### 7.4 Export/import (FR-F5, FR-F7)

`.argon` is a zip (crate `zip`) of the cart folder (or the single file wrapped
in a folder with a generated manifest). Text export (P1) is a UTF-8 document
with `== file: name ==` sections and base64 for binaries; small and
paste-friendly. Import validates the manifest, rejects path escapes in zip
entries, and refuses to overwrite without confirmation.

## 8. Shell and editor (argon-shell)

Both are Rust structs implementing

```rust
trait App {
    fn handle_key(&mut self, key: KeyEvent, grid: &mut TextGrid) -> Transition;
    fn tick(&mut self, grid: &mut TextGrid);   // cursor blink, etc.
}
```

and draw only through `TextGrid`. They run on the main thread, so a keystroke
is handled and rendered in the same frame (NFR-2).

Shell: a line editor with history and tab completion; a command parser that
tries the command table first and falls back to immediate-mode Lua (FR-S3).
Immediate mode uses a long-lived `argon-lua` instance with a special "shell
program" whose `flip()` is a no-op and whose draws go straight to the shell's
framebuffer; its sandbox jail is `disk/`.

Editor: a `ropey`-backed buffer, undo stack of edit operations, Lua
tokenizer for highlighting (hand-written, a few hundred lines, no tree-sitter
dependency), viewport with vertical and horizontal scroll, and a snippet
engine for placeholders (FR-E2). Menus are overlays drawn on the grid; the
Catalog is a filterable list reading `ApiTable`.

Decision: shell and editor are Rust, not Lua carts. Dogfooding the API by
writing them in Lua is attractive and may happen later, but v1 prioritizes
latency and robustness, and the editor needs features (clipboard, undo) that
would otherwise force those into the public API prematurely.

## 9. Binary, CLI and headless (argon, FR-T1, FR-T2)

```
argon                      # GUI, boot to shell
argon run CART [--frames N] [--screenshot out.png] [--headless] [--input rec.json]
argon check CART           # parse Lua, compile shaders, validate manifest
argon docs [--format md]   # API reference from ApiTable
argon --disk PATH | --portable
```

Headless: no winit window; wgpu is created from an adapter with no surface
and renders into the framebuffer texture; `--screenshot` reads it back after
N frames. Input comes from a recording or is empty. This is how CI runs the
demos suite and how development on a display-less machine works.

Note for this devbox: wgpu with no display needs a Vulkan ICD or GL
implementation; verify in Phase 0 whether a software driver (lavapipe/
swiftshader) is available or installable without root. If not, the fallback
is a wasm build run in headless Chromium with the already-proven Playwright
workflow (WebGPU via swiftshader is the open question there).

## 10. Determinism (FR-R9 to FR-R11, NFR-8)

- `random` is a PCG32 seeded from `seed()` or from the frame counter at start.
- `time()` and `delta_time()` derive from the main thread's frame clock; in
  deterministic mode they derive from `frame_no / fps`.
- `InputSnapshot` is the only external input; recording it per frame gives
  replay for free once the above holds.
- Tier 0/1 output is produced by fixed-point tessellation on the CPU and
  integer pixel coverage rules, so it does not depend on the GPU vendor.
  Shaders are explicitly excluded from cross-machine determinism.

## 11. Browser feasibility (D2, NFR-5)

What the desktop design assumes that wasm cannot do by default: a program
thread that blocks in `flip()`.

Options, in order of preference:

1. Web Worker + `SharedArrayBuffer` + `Atomics.wait`. Luau compiled to wasm
   runs in the worker; `flip()` posts the frame and blocks on an atomic the
   main thread sets. Requires cross-origin isolation headers on the hosting
   page, which Argon's own export page controls. This preserves the exact
   desktop architecture.
2. Asyncify the Lua interpreter so `flip()` unwinds the C stack to the
   browser event loop and resumes next frame. Works without COOP/COEP but
   costs speed and binary size.

wgpu targets WebGPU directly; naga runs in wasm. cpal has a WebAudio backend.
`argon-core`'s renderer-agnostic `DrawCmd` and the channel-based protocol
make option 1 a transport swap rather than a redesign. Nothing in v1 is
allowed to introduce `std::thread`-only assumptions outside the small
"transport" module.

## 12. Key dependencies

| Need | Crate | Notes |
|------|-------|-------|
| Window/events | winit 0.30 | Keyboard-only; no cursor (hide cursor over window). |
| GPU | wgpu 24+ | Vulkan/Metal/DX12/GL/WebGPU. |
| Shader frontend | naga (glsl-in) | Ships with wgpu. |
| Lua | mlua (luau, vendored) | Fallback lua54. |
| Audio | cpal, rtrb, hound, lewton | |
| Images | image (png only) | |
| Text buffer | ropey | Editor. |
| Zip | zip | Export/import. |
| Config | serde, toml | |
| Paths | directories | `~/.argon`. |
| CLI | clap | |
| Errors | thiserror, anyhow | |
| Tests | insta (snapshot), image diff | Headless screenshots. |

Dependency count is kept deliberately low; every addition must justify
itself against the 30 MB binary budget (NFR-4) and the wasm target.

## 13. Risks and Phase 0 spikes

| Risk | Impact | Spike / mitigation |
|------|--------|--------------------|
| naga GLSL frontend rejects common Shadertoy code | D6 at risk | Run 20 popular shaders through naga in week 1; document failures; consider glslang-via-wasm fallback if >30% fail. |
| Luau interrupt latency or mlua luau quirks | FR-R6 | Prototype break-from-tight-loop on day 1 of the Lua work; measure. |
| Headless wgpu on display-less dev machine | Dev velocity on this devbox | Try `WGPU_BACKEND=vulkan` with lavapipe, then GL via OSMesa, then wasm fallback. |
| Channel round trip per flip adds latency | Feel | Measure; expected < 0.2 ms. If needed, use a `parking_lot` condvar instead of channels. |
| Luau memory limit granularity | FR-R7 | Test allocation storms; ensure the error is catchable by Argon, not the program. |
| Text layer + framebuffer compositing order confuses users | UX | Decide and document: text always above pixels; `cls` clears both unless given a layer argument. |
| Font coverage for non-Latin names | Accessibility | Ship Latin-1 + box drawing in v1; add a fallback TTF path (fontdue) in v1.x. |

## 14. Phase 0 build order

1. Workspace, `argon-core` types, `TextGrid`, bitmap font asset.
2. `argon-gfx`: window, presenter, text layer draw. Visible: a blinking
   `ARGON>`.
3. Shell: line editor, `DIR CLS HELP VER`, VFS with jail.
4. `argon-lua`: sandboxed state, `print`, immediate mode in the shell.
5. Program thread, `flip()`, break flag, `cls pixel line fill_rect`,
   `button`. First Pong.
6. Editor: buffer, viewport, save, F5 run, error cursor jump.
7. Headless `argon run --screenshot` and a CI job that renders a demo.
8. Spikes: naga Shadertoy compatibility; Luau interrupt latency.
