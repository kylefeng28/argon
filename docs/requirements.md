# Argon - Product Requirements

## 1. Vision

Argon is a fantasy console, inspired by the design of retro computers such as the the BBC Micro, Commodore 64, ZX Spectrum,
as well as the aesthetics of CRTs and vector displays.

Argon recreates the two properties that made these retro computing devices so approachable to users:
1) zero distance between having an idea, writing the code, and running the code, and 2) a well curated set of builtin commands that can be easily discovered and learned, and combined to form complex programs.

Programs run in a sealed sandbox with their own shell, display, input, audio and disk, but the display is backed by a real GPU
and the runtime is backed by a real CPU. Thus, Argon provides the simplicity of retro computers as well as the speed and performance of modern computers.

A user should be able to:

- launch one binary and be at a prompt in under a second,
- type `NEW pong`, press F5, and be running code within a minute,
- discover every available function from a menu rather than documentation,
- grow a program from text output to pixels to shaders to render-to-texture
  without ever changing tools, and
- share the result as a single file that another Argon user can run.

## 2. Decisions log

| # | Topic | Decision |
|---|-------|----------|
| D1 | Scripting language | Lua only at launch. A C subset via a WebAssembly ABI may come at a future phase; the host API is designed language-neutral from day one. |
| D2 | Platforms | Desktop (Windows, macOS, Linux) is the launch target. Browser (wasm + WebGPU) is important but secondary; nothing in the design may preclude it. |
| D3 | Display | Programs default to 1280x800 (`hd`); `native` (the window's pixels) is opt-in per cart, and bare `.glsl` programs default to native. The shell and editor use the largest 16:10 multiple of 640x400 that fits the window and keep an 80x25 text grid at every size by scaling the 8x16 cell (2x at 800 rows, 3x at 1200). The pixel count rises with the display; the interaction model does not. |
| D4 | Color | Per-cart setting: true color (default) or an indexed palette of 2 to 256 entries. |
| D5 | Pointer | No mouse cursor anywhere in the UI. Mouse events may be exposed to programs later in the style of xterm/kitty mouse reporting. Shell and editor are keyboard-only. |
| D6 | Shader language | GLSL, Shadertoy copy-paste compatible (`mainImage`, `iTime`, `iResolution`, `iChannel0..3`, ...). |
| D7 | Program model | Linear scripts. A program runs top to bottom and calls `flip()` to present a frame. No mandatory `update`/`draw` callbacks. |
| D8 | Audio | Sample playback (WAV/OGG) plus a small synthesizer. No tracker at launch. |
| D9 | Sharing | Whichever cart format is simplest to implement, but exporting must be one command. |
| D10 | Audience priority | Power features and speed first. Error messages must still be easy to read, and API names are explicit (`fill_circle`, not `circfill`). |
| D11 | Multitasking | Strictly one program at a time. Esc returns to the shell. |
| D12 | License | Open source. |
| D13 | Lua engine | Luau (via mlua). |
| D14 | Pad key mapping | Default: arrows, Z=a, X=b, A=x, S=y, Q=l, W=r, Enter=start, RShift=select. All bindings configurable in `~/.argon/config.toml`. |
| D15 | Deletion | `DEL` requires confirmation. No trash folder. |
| D16 | Immediate mode state | The shell's Lua state persists across `RUN`s. |

## 3. Personas

- The nostalgist. Senior engineer who wants to tinker in the evening without
  installing a toolchain. Values instant boot, no project setup, and power on
  demand. Primary persona.
- The jammer. Game-jam or demoscene participant. Values shaders, audio,
  deterministic replay, one-file export.
- The learner. Teenager or adult learning to program. Values discoverable
  menus, readable errors, instant visual feedback, inability to break anything.
  Secondary persona; we do not compromise power for them, but we do not make
  things gratuitously hostile either.
- The teacher. Wants a classroom-safe sandbox with no host or network access
  and a printable API reference. Served as a side effect of the sandbox and
  the docs requirements.

## 4. Design principles

1. Boot to prompt. No wizard, no project, no account.
2. Finite, browsable vocabulary. Every API function, shell command and
   language construct is reachable from a menu and from `HELP`.
3. Progressive power. The first functions a user meets are `print`, `cls`,
   `pixel`. Shaders and render targets exist but never get in the way.
4. The sandbox is the product. Programs can only touch the virtual machine.
   A broken program never breaks the console.
5. One file is a program. A `.lua` file on the virtual disk is runnable.
   Folders with assets and metadata are optional.
6. Linear by default. Programs read like a TI-84 or DOS BASIC listing: set
   up, loop, draw, `flip()`, repeat.
7. Explicit names, short names. `fill_circle`, `draw_sprite`, `wait_key`.
   No abbreviations that need a glossary.

## 5. Functional requirements

Priority: P0 = required for v1 launch, P1 = v1 if time permits, otherwise v1.x,
P2 = later phase.

### 5.1 Console and display

| ID | Priority | Requirement |
|----|----------|-------------|
| FR-C1 | P0 | Argon is a single executable that opens one window and boots directly into the shell. |
| FR-C2 | P0 | The virtual display has a per-cart resolution, default 1280x800. Presets: `tiny` 128x128, `retro` 320x240, `classic` 640x400, `hd` 1280x800, `uhd` 2560x1600, `native` (the window). Arbitrary `WxH` is also accepted. `.glsl` programs default to `native`. |
| FR-C3 | P0 | The virtual display is integer-scaled to the largest multiple that fits the window, centered, with letterbox bars. Nearest-neighbor sampling. |
| FR-C4 | P1 | Optional display filters selectable per cart or by the user: none, glow (halo around strokes, white-hot core), glass (specular rim and dispersion, the light-ribbon look), scanlines, neon (glow + glass + vignette, default), crt (glow + scanlines + vignette). Implemented in the presenter shader; headless screenshots at scale >= 2 include them. |
| FR-C5 | P0 | Per-cart color mode: `truecolor` (default) or `palette = N` (2 <= N <= 256) with a user-defined palette. In palette mode color arguments are indices; in truecolor they are packed RGBA or `rgb()` values. |
| FR-C6 | P0 | A text layer with a fixed character grid overlays the pixel framebuffer. The base cell is 8x16, scaled by round(height / 400) so every 16:10 resolution gives 80x25 cells (640x400 at 1x, 1280x800 at 2x, 1920x1200 at 3x). The layer is rasterized at the window's integer scale from a TrueType face (Share Tech Mono by default, ProggyClean optional) with the Unscii bitmap font as fallback for box-drawing and symbols, so UI text is sharp while program pixels stay chunky. The shell and editor use the same text layer as programs. |
| FR-C7 | P0 | Fullscreen toggle and window scaling without affecting the cart's virtual resolution. |
| FR-C8 | P0 | The console never shows a mouse cursor. |
| FR-C9 | P0 | User-selectable UI theme for shell and editor persisted across launches. Default is `grid`: Tron: Legacy electric cyan on near-black blue with orange for errors and strings. Also: green phosphor, amber, blue, gray LCD. |
| FR-C10 | P0 | Alternate screen: `alt_screen()` switches a program to a fresh framebuffer and text layer; the shell's screen (pixels and text) is restored when the program ends or calls `main_screen()`. Programs that do not call it leave their output on the shell screen, DOS-style. |

### 5.2 Shell

| ID | Priority | Requirement |
|----|----------|-------------|
| FR-S1 | P0 | Prompt is `ARGON>` followed by the current directory when not at root, e.g. `ARGON:GAMES>`. |
| FR-S2 | P0 | Commands (case-insensitive): `DIR` (alias `LS`), `RUN name`, `EDIT name`, `NEW name`, `COPY a b`, `REN a b`, `DEL name`, `TYPE name`, `CLS`, `HELP [topic]`, `CD dir`, `MKDIR dir`, `EXPORT name`, `IMPORT path`, `VER`, `THEME name`, `EXIT`. |
| FR-S3 | P0 | Any input that does not parse as a shell command is evaluated as Lua in a persistent immediate-mode state. Expressions print their value; statements execute. Drawing functions work and draw on the shell screen. |
| FR-S4 | P0 | Tab completion for commands, file names and global Lua names. Up/Down recall history (persisted across launches). |
| FR-S5 | P0 | `HELP` lists command and API categories. `HELP <name>` shows the signature, one-line description and an example for any shell command or API function. |
| FR-S6 | P0 | `RUN` with no argument re-runs the last program. `EDIT` with no argument reopens the last edited file. |
| FR-S7 | P0 | After a program ends or errors, the shell shows the exit reason: normal completion, user break, or the error with file and line. |
| FR-S8 | P0 | Boot banner showing version, disk usage and a hint (`Type HELP to begin`). |
| FR-S9 | P1 | `DIR` shows carts (folders with `cart.toml`) distinctly from plain files and folders, with title from the manifest. |
| FR-S10 | P2 | `RECORD name` / `PLAY name` for input recording and deterministic replay. |

### 5.3 Editor

| ID | Priority | Requirement |
|----|----------|-------------|
| FR-E1 | P0 | Full-screen text editor on the text layer: syntax highlighting for Lua, line numbers, undo/redo, cut/copy/paste within the console, find and replace, auto-indent, bracket matching, soft wrap off, horizontal scroll. |
| FR-E2 | P0 | Function-key menus that insert snippets: F2 Control, F3 I/O, F4 Draw, F6 Math, F7 Sound, F8 Files, F9 GPU. Snippets contain placeholders that Tab cycles through. |
| FR-E3 | P0 | F1 opens the Catalog: an alphabetical, filterable list of every API function with signature, description and example. Enter inserts the call at the cursor. |
| FR-E4 | P0 | F5 saves and runs the current program. On error, Argon returns to the editor with the cursor on the offending line and the message in the status line. Esc from a running program returns to the editor if launched from the editor, otherwise to the shell. |
| FR-E5 | P0 | F10 (or Esc with confirmation when modified) saves and returns to the shell. Ctrl+S saves. |
| FR-E6 | P0 | Status line shows file name, modified flag, line:col, and the last message. |
| FR-E7 | P1 | Tabs for multi-file carts (`main.lua` plus modules). Single-file programs show no tabs. |
| FR-E8 | P1 | Go to line, select all, duplicate line, move line up/down, toggle comment. |
| FR-E9 | P2 | Live mode: hot reload on save for carts that opt in. |
| FR-E10 | P2 | Sprite and sound editors as additional editor modes. |

### 5.4 Runtime and language

| ID | Priority | Requirement |
|----|----------|-------------|
| FR-R1 | P0 | Lua runtime with `string`, `table`, `math`, `utf8`, `coroutine` and the base library minus `dofile`, `loadfile`, `loadstring` of binary chunks, `os`, `io`, `debug`, `package`. |
| FR-R2 | P0 | `require("name")` resolves only within the running cart's folder on the virtual disk. |
| FR-R3 | P0 | Linear program model: the script runs top to bottom. `flip()` presents the frame, processes input, enforces the frame rate, and returns. A program that returns from the top level ends normally. |
| FR-R4 | P0 | Blocking convenience calls that internally flip: `input()`, `wait_key()`, `wait(seconds)`. |
| FR-R5 | P0 | Target frame rate is configurable per cart (default 60). `flip()` sleeps to hit it; `delta_time()` reports the actual elapsed time. |
| FR-R6 | P0 | Esc interrupts any program within 100 ms, including a program stuck in a loop that never calls `flip()`. The interrupted program is torn down completely (Lua state, GPU resources, audio voices). |
| FR-R7 | P0 | Per-program memory limit (default 512 MB, configurable) enforced by the Lua allocator. Exceeding it raises a readable error. |
| FR-R8 | P0 | Runtime errors report `file:line: message`, the source line, and for unknown globals a "did you mean" suggestion from the API table. Shader compile errors are reported in the same format against the shader source. |
| FR-R9 | P0 | Argon-provided `random()` uses its own seedable PRNG so programs are reproducible. |
| FR-R10 | P0 | `time()` returns seconds since program start; `frame()` returns the frame counter. No wall clock or date is exposed in v1. |
| FR-R11 | P1 | Deterministic mode: fixed timestep, frame-derived `time()`, recorded input. Prerequisite for FR-S10. |
| FR-R12 | P2 | WebAssembly cart ABI (`main.wasm` in a cart) with the same host API, enabling a C subset and external languages. |

### 5.5 Graphics API

Tiers are a naming convention for documentation and menus; all tiers are in
one API.

| ID | Priority | Requirement |
|----|----------|-------------|
| FR-G1 | P0 | Tier 0 text: `print`, `write`, `locate`, `text_color`, `cursor`, `put_char`, `text_size`, `cls`. The text layer scrolls when `print` passes the last row. |
| FR-G2 | P0 | Tier 1 primitives: `pixel`, `get_pixel`, `line`, `rect`, `fill_rect`, `circle`, `fill_circle`, `ellipse`, `fill_ellipse`, `triangle`, `fill_triangle`, `polygon`, `fill_polygon`, `draw_text` (pixel text at any x,y), `clip`, `camera`, `color`. |
| FR-G3 | P0 | Tier 1 images: `load_image(path)` from PNG on the virtual disk, `new_image(w,h)`, `draw_image`, `draw_sprite` (sub-rectangle with flip, scale, rotation, tint, alpha), `image_size`. |
| FR-G4 | P0 | Render to texture: `set_target(img)` redirects all drawing to an image; `set_target()` restores the screen. |
| FR-G5 | P0 | Blend modes: `alpha` (default), `add`, `multiply`, `replace`. |
| FR-G6 | P0 | Tier 2 shaders: `load_shader(src)` accepts a Shadertoy-style GLSL fragment shader. `draw_shader(sh, opts)` runs it over the screen or a rectangle. Uniforms `iResolution`, `iTime`, `iTimeDelta`, `iFrame`, `iMouse` (zeros until mouse support), `iChannel0..3` are provided; user uniforms are passed as a table. |
| FR-G7 | P0 | `set_shader(sh)` applies a user fragment shader to subsequent sprite and primitive draws (for palette swaps, outlines, lighting). |
| FR-G8 | P0 | Framebuffer persists between flips. `cls` is the only thing that clears it, matching the DOS/TI-84 model. |
| FR-G9 | P0 | Performance: at `classic` resolution, 100,000 `draw_sprite` calls per frame at 60 FPS on a 2020 integrated GPU; a full-screen Shadertoy shader at native 1080p at 60 FPS on a mid-range discrete GPU. |
| FR-G10 | P1 | `get_pixel` and `save_image(img, path)` (PNG) read back GPU data; documented as slow-path operations. |
| FR-G11 | P2 | Tier 3: cameras, meshes, transforms, default lit material, custom vertex shaders. |
| FR-G12 | P2 | Tier 4: compute shaders over buffers. |

### 5.6 Input

| ID | Priority | Requirement |
|----|----------|-------------|
| FR-I1 | P0 | Abstract pad: `button(name)`, `button_pressed(name)`, `button_released(name)` for `up down left right a b x y l r start select`, keyboard-mapped by default, remappable. |
| FR-I2 | P0 | Raw keys: `key(name)`, `key_pressed(name)`, `key_released(name)` with portable key names. |
| FR-I3 | P0 | Text input: `read_key()` pops the next typed character or special key (non-blocking, nil when empty); `wait_key()` blocks; `input(prompt)` reads a line with editing. |
| FR-I4 | P0 | Input state is sampled once per `flip()` and is stable for the whole frame. |
| FR-I5 | P1 | Gamepad support mapped onto the abstract pad; up to 4 players. |
| FR-I6 | P2 | Mouse events reported to programs (position, buttons, wheel) in the terminal-emulator style; no cursor is ever drawn by Argon. Implemented: `mouse`, `mouse_button`, `mouse_pressed`, `mouse_released`, `mouse_wheel`; the shell and editor ignore the mouse. |

### 5.7 Audio

| ID | Priority | Requirement |
|----|----------|-------------|
| FR-A1 | P0 | `load_sound(path)` for WAV and OGG Vorbis from the virtual disk; `play_sound(snd, opts)` with volume, pan, pitch, loop; `stop_sound`. At least 16 simultaneous voices. |
| FR-A2 | P0 | Synth: `beep(freq, duration, opts)` and `tone(channel, opts)` with square, triangle, saw, sine and noise waveforms, volume envelope, duty cycle. 8 synth channels. |
| FR-A3 | P0 | `set_volume(master)`; sound is cut immediately on program exit or break. |
| FR-A4 | P1 | `play_notes("C4 E4 G4 C5", opts)` simple note sequencer. |
| FR-A5 | P2 | Tracker and sound editor. |

### 5.8 Virtual disk, carts and sharing

| ID | Priority | Requirement |
|----|----------|-------------|
| FR-F1 | P0 | The virtual disk is a plain host directory (default `~/.argon/disk`, overridable by `--disk` and a portable mode that uses `./disk` next to the executable). Files are ordinary files so git and sync tools work. |
| FR-F2 | P0 | Programs can read and write only inside their own cart folder and the shared `SAVE/` folder. Path traversal and symlinks out of the disk are rejected. |
| FR-F3 | P0 | A program is either a single `name.lua` file or a cart folder containing `cart.toml` and `main.lua`. The manifest holds title, author, resolution, color mode, fps, api version. |
| FR-F4 | P0 | Program file API: `read_file`, `write_file`, `append_file`, `file_exists`, `delete_file`, `list_files`, and a key-value store `save(key, value)` / `load(key)` namespaced per cart. |
| FR-F5 | P0 | `EXPORT name` writes `name.argon` (a zip of the cart) to `~/.argon/export/` and prints the host path. `IMPORT path` copies an `.argon` file or a `.lua` file into the disk. |
| FR-F6 | P0 | Argon ships with a read-only `DEMOS/` folder containing at least 12 programs covering every P0 API area; each is under 120 lines and commented. |
| FR-F7 | P1 | `EXPORT name --text` emits a single UTF-8 text file with code and base64 assets that `IMPORT` accepts, for pasting into chats and forums. |
| FR-F8 | P2 | Export as a standalone web page (cart + wasm runtime). |

### 5.9 Tooling and headless use

| ID | Priority | Requirement |
|----|----------|-------------|
| FR-T1 | P0 | `argon run <cart> [--frames N] [--screenshot path] [--headless]` runs a cart without the shell; headless mode renders offscreen and exits. Used for CI and for the demos regression suite. |
| FR-T2 | P0 | `argon check <cart>` parses the Lua and compiles any shaders without running. |
| FR-T3 | P1 | `argon docs` prints the API reference as Markdown generated from the same table that feeds the editor Catalog. |

## 6. Nonfunctional requirements

| ID | Requirement |
|----|-------------|
| NFR-1 Startup | Cold start to prompt under 1 s on a 2020 laptop; warm under 300 ms. |
| NFR-2 Input latency | Keystroke to glyph in shell and editor within one frame at the display refresh rate. This is the primary feel metric and is measured in CI with a frame-timestamped test. |
| NFR-3 Throughput | FR-G9 numbers are published and tracked; regressions over 10% fail CI. |
| NFR-4 Footprint | Single static binary under 30 MB. No installer, no runtime dependencies beyond the OS graphics driver. |
| NFR-5 Portability | Windows 10+, macOS 12+, Linux (X11 and Wayland) at launch from one code base. A cart behaves identically on all. Browser build must remain feasible: no threads-with-blocking assumptions that WebAssembly cannot satisfy without a fallback. |
| NFR-6 Isolation | Programs have no access to host files, network, environment variables, wall clock, or processes. Enforced in the runtime, tested by an adversarial cart suite. Threat model: an untrusted cart from a stranger. |
| NFR-7 Robustness | Infinite loops, deep recursion, memory exhaustion, invalid GPU usage and shader compile failures inside a cart never crash or hang Argon. Esc works within 100 ms in all cases. |
| NFR-8 Determinism | Given the same cart, seed and input log, Tier 0 and Tier 1 output is identical across runs and platforms. Shader output is identical across runs on the same machine. |
| NFR-9 Discoverability | Every public API function appears in exactly one editor menu category and in the Catalog with a working example. A CI check fails on an undocumented function. |
| NFR-10 Compatibility | Carts declare `api = 1`. Later Argon versions run `api = 1` carts unchanged. |
| NFR-11 Accessibility | UI scale, font and theme are configurable; all keys remappable; no information conveyed by color alone in shell or editor. |
| NFR-12 Offline | Fully functional offline. No telemetry, no update checks without opt-in. |
| NFR-13 Testability | Headless mode (FR-T1) exercises the full renderer so screenshots and recorded-input replays can be compared in CI, including on developer machines without a display. |
| NFR-14 Code quality | Rust stable, `clippy` clean, `cargo test` green on all three desktop platforms in CI. |

## 7. User stories

Format: As a [persona], I want [goal], so that [reason]. Acceptance criteria
follow. Priorities match section 5.

### Epic A: Boot and shell

- A1 (P0) As a nostalgist, I want to double-click one file and see a prompt,
  so that starting costs nothing.
  - Window opens with banner and `ARGON>` in under 1 s.
  - No files are created outside `~/.argon`.
- A2 (P0) As a learner, I want to type `HELP` and see what I can do, so that
  I never have to leave the console to learn it.
  - `HELP` lists categories; `HELP fill_circle` shows signature, description,
    example; `HELP RUN` shows the command.
- A3 (P0) As a nostalgist, I want to type `2^10` or `fill_circle(40,40,20,
  rgb(255,0,0))` at the prompt and see the result immediately, so that the
  shell doubles as a calculator and scratchpad.
  - Expression value is printed; drawing calls draw on the shell screen;
    errors print in the standard error format.
- A4 (P0) As any user, I want `DIR`, `COPY`, `REN`, `DEL`, `TYPE` to work as
  I expect from DOS, so that managing programs needs no new learning.
  - Each command has `HELP`; `DEL` asks for confirmation; wildcards `*` and `?`
    are supported in `DIR`, `COPY`, `DEL`.
- A5 (P0) As any user, I want Esc to get me back to the shell from anything,
  so that I am never stuck.
  - From a running program (including a tight loop with no `flip()`), Esc
    returns within 100 ms with the break message shown.

### Epic B: Editing

- B1 (P0) As a nostalgist, I want `NEW pong` to drop me into an editor with a
  minimal template, so that I start from running code.
  - Template contains a comment header, `cls()`, a loop with `flip()`.
- B2 (P0) As a learner, I want to press F4 and pick `fill_circle` from a menu,
  so that I do not have to remember the name or argument order.
  - Snippet `fill_circle(x, y, r, color)` is inserted with `x` selected; Tab
    moves through placeholders.
- B3 (P0) As any user, I want F5 to save and run, and on error to land on the
  broken line with the message visible, so that the edit-run loop is one key.
- B4 (P0) As a power user, I want find/replace, undo/redo and go-to-line, so
  that the editor is usable for 1,000-line programs.
- B5 (P0) As a learner, I want F1 to open a searchable catalog of every
  function with an example, so that I can browse what exists.
- B6 (P1) As a jammer, I want tabs for a multi-file cart, so that I can split
  a larger game into modules.

### Epic C: Running programs

- C1 (P0) As a nostalgist, I want to write `while true do ... flip() end` and
  have it run at a steady 60 FPS, so that programs read like BASIC listings.
  - `flip()` paces to the cart's fps; `delta_time()` is accurate.
- C2 (P0) As a learner, I want `name = input("Your name? ")` to pause and read
  a line, so that text programs are trivial.
- C3 (P0) As any user, I want a clear error such as
  `main.lua:12: attempt to call a nil value (global 'fil_rect'). Did you mean fill_rect?`
  with the source line shown, so that I fix typos in seconds.
- C4 (P0) As a teacher, I want an untrusted cart to be unable to read my home
  directory or reach the network, so that I can run student submissions
  safely.
  - Adversarial test carts attempting `io`, `os`, `..` paths, `require` of
    absolute paths and huge allocations all fail with errors, never with
    access.
- C5 (P0) As a jammer, I want `random()` to be seedable, so that procedural
  content is reproducible.
- C6 (P1) As a jammer, I want to record my inputs and replay them to a
  bit-identical result, so that I can debug and capture footage reliably.

### Epic D: Graphics

- D1 (P0) As a nostalgist, I want pixel, line, rect and circle primitives on a
  persistent framebuffer, so that classic graphics programs port directly.
- D2 (P0) As a jammer, I want to load PNG sprite sheets and draw sub-sprites
  with flip, scale, rotation and tint, so that 2D games are practical.
- D3 (P0) As a power user, I want to draw 100,000 sprites a frame at 60 FPS,
  so that particle systems and bullet-hell games are possible without tricks.
- D4 (P0) As a jammer, I want to paste a Shadertoy shader into a string and
  `draw_shader` it full screen, so that I get the GPU without learning a new
  dialect.
  - The top 20 Shadertoy shaders that use only `mainImage` and standard
    uniforms run unmodified.
- D5 (P0) As a power user, I want `set_target(img)` to render into an image
  and then draw that image with a shader, so that post-processing and
  lighting are possible.
- D6 (P0) As a jammer, I want to choose 320x180 with a 16-color palette for
  one cart and 1280x720 true color for another, so that each project has its
  own look.
- D7 (P2) As a power user, I want meshes, cameras and custom vertex shaders,
  so that I can do 3D.

### Epic E: Sound and input

- E1 (P0) As a learner, I want `beep(440, 0.2)` to make a sound, so that
  audio is one line.
- E2 (P0) As a jammer, I want to load and play WAV/OGG with volume, pan and
  pitch, so that games have real sound effects and music.
- E3 (P0) As any user, I want `button("left")` and `button_pressed("a")`
  with remappable keys, so that controls are consistent across carts.
- E4 (P1) As a jammer, I want gamepads to map onto the same buttons, so that
  games play from the couch.

### Epic F: Sharing

- F1 (P0) As a jammer, I want `EXPORT pong` to produce one file I can send to
  a friend, who runs `IMPORT` and `RUN pong`, so that sharing is trivial.
- F2 (P0) As a learner, I want a `DEMOS` folder I can read, run and copy from,
  so that I learn by example.
- F3 (P1) As a nostalgist, I want a text-only export I can paste into a
  forum post, so that programs can be shared like magazine listings.
- F4 (P2) As a jammer, I want to export a web page that runs my cart, so that
  non-Argon users can play it.

## 8. Out of scope (v1)

- C or any non-Lua language (D1).
- Mouse cursor; mouse events (D5).
- Networking from carts.
- Package manager, plugins, extension API, LSP.
- Scene editor, physics, ECS. These can be written in Lua by users.
- Running more than one program at once (D11).
- Tracker, sprite editor, sound editor.
- Browser build as a supported release (kept feasible, not shipped).

## 9. Phasing

- Phase 0, spike (2 weeks). wgpu window, text layer, shell with `DIR RUN EDIT
  NEW CLS`, minimal editor, Lua with `print cls pixel line fill_rect flip
  button`, Esc break, headless screenshot. Exit criterion: Pong written inside
  Argon, and the headless runner produces a screenshot in CI with no display.
- Phase 1, v1 (launch). All P0 requirements. Demos folder. Three-platform CI.
- Phase 2, v1.x. P1 items: gamepads, multi-file editor, text export,
  deterministic record/replay, display filters, `argon docs`.
- Phase 3, v2. Tier 3 and 4 graphics, wasm cart ABI and a C subset, browser
  build, live reload, asset editors.

## 10. Success metrics

- Time from first launch to a running user-written program: median under 5
  minutes in hallway tests with engineers, under 15 minutes with learners.
- Pong, Breakout, a Shadertoy port and a 100k-particle demo each fit in under
  150 lines of Lua in `DEMOS/`.
- NFR-1, NFR-2 and FR-G9 numbers measured and published in the README.
