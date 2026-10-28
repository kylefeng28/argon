# Argon

A fantasy console. Embrace the soft flickering glow of neon vacuum tubes.

## Inspiration
- [Uxn](https://100r.co/site/uxn_design.html)/[Varvara](https://100r.co/site/uxn.html) by Hundred Rabbits
- DCPU from [0x10c](https://en.wikipedia.org/wiki/0x10c)
- Retro systems like Commodore 64 and ZX Spectrum

## Status

Phase 0 is implemented (see `docs/requirements.md` section 9 for phasing):

- Boots to an `ARGON>` prompt. Shell commands `DIR RUN HELP NEW EDIT TYPE COPY REN
  DEL CD MKDIR CLS THEME VER EXPORT IMPORT EXIT`, tab completion,
  history, and immediate-mode Lua (`EVAL 2^10`, `LUA fill_circle(320, 200, 50)`).
- Full-screen editor: syntax highlighting, undo/redo, selection, find/replace,
  go to line, F1 catalog and F2/F4/F6/F7/F8/F9 insert menus with tab-through
  placeholders, F5 save-and-run with the cursor jumping to the error line.
- Sandboxed Luau runtime (native code generation on, optimization level 2;
  `ARGON_NO_JIT=1` to compare) on its own thread with the linear `flip()` model,
  Esc break within 100 ms, memory limit, readable errors with "did you mean"
  hints, seedable PRNG, per-program file jail, `save`/`load`, and
  `alt_screen()` so full-screen programs leave the shell untouched.
- Shaders: Shadertoy-style GLSL fragment shaders compiled with naga at
  runtime (`mainImage`, `iTime`, `iMouse`, `iChannel0..3`, your own
  `uniform` declarations); run full-screen, over a rect, or applied to sprite
  batches; errors report the shader's own line numbers.
- Audio: a chip-style synth (`beep`, `tone` on 8 channels, square/triangle/
  saw/sine/noise, envelopes, slides) plus WAV/OGG playback through cpal.
- wgpu renderer: persistent 640x400 framebuffer (per-cart resolution),
  pixel-exact CPU-rasterized primitives, sprites with rotation/scale/flip,
  render-to-texture, blend modes, clip, camera, 8x16 text layer, integer-scaled
  presentation with display filters (glow (bloom halo), scanlines, vignette),
  and a headless mode for CI.
- Appearance
  - Themes:
    - 4 retro themes: amber/phosphor/blue/lcd
    - `grid` theme inspired by Tron: Legacy electric cyan on near-black blue with orange accents, plus a neon glow filter.
  - `FILTER` presets: none/glow/scanlines/neon/crt
  - Filters: The `neon` display filter gives bright strokes a white-hot core, a colored halo and a glass-like specular rim, the light ribbon look; `glow`, `glass`, `scanlines` and `crt` are also available (`FILTER`).
  - Font: Text is Share Tech Mono rasterized at the window's scale (so it stays sharp), with ProggyClean and the Unscii bitmap font as alternatives (`FONT`).
- 28 demo programs in `DEMOS/`: GPU Mandelbrot and Julia, a raymarched SDF scene, shader post-processing, light cycles, Game of Life, flocking boids, strange attractors, a progressive Mandelbrot and an animated Julia set, a tactical HUD, matrix rain, the Tetris theme as a chiptune, a software-rendered 3D cube, and more.

Not yet: palette mode, gamepads, `.argon` zip export, browser build. See `docs/api-sketch.md` for the API.

## Build and run

Requires stable Rust, a C++ compiler (for Luau) and, on Linux, the ALSA
development package (`libasound2-dev` / `alsa-lib-devel`) for audio.

```
cargo run --release              # windowed
cargo run --release -- run DEMOS/pong
cargo run -- docs > API.md       # API reference from the live table
```

Keys:
- `F11` or `Alt+Enter` toggles fullscreen.
- Esc stops a program.
- In the editor, F1 is the catalog, F5 runs, F10 saves and exits.
- Pad: arrows, Z/X/A/S, Q/W, Enter, Shift (configurable in `~/.argon/config.toml`).

Data lives in `~/.argon/` (`disk/` is the virtual disk; `--disk PATH` or `--portable` override).

## Headless and tests

```
cargo test --workspace
source tools/headless-env.sh      # on a machine without a GPU/display
tools/smoke.sh                    # run every demo and screenshot shell/editor
./target/debug/argon shell --headless --keys "help{enter}" --text --screenshot out.png
./target/debug/argon run DEMOS/shapes --headless --frames 30 --screenshot shapes.png
```

The GPU tests and the headless runner need any Vulkan/Metal/DX12 adapter; a
software one is fine. `tools/headless-env.sh` points wgpu at the SwiftShader
driver bundled with Playwright's Chromium if present.

## Documents

- `docs/manual.md` - the user manual (shell, editor, programs, files)
- `docs/agent-guide.md` - compact rules and reference for code-generating
  models; paste it into a prompt to have a model write Argon programs
- `docs/requirements.md` - vision, decisions, functional and nonfunctional
  requirements, user stories, phasing
- `docs/technical-design.md` - architecture
- `docs/api-sketch.md` - the Lua API surface for Tiers 0 to 2

## Layout

```
crates/argon-core   colors, font, text grid, draw commands, input, VFS, config
crates/argon-gfx    wgpu renderer (framebuffer, text layer, presenter, headless)
crates/argon-audio  mixer, chip synth, WAV/OGG decoding, cpal output
crates/argon-lua    Luau sandbox, API bindings + docs table, program thread
crates/argon-shell  console state machine, shell, editor, menus
crates/argon        binary: winit app, headless runner, CLI
disk/DEMOS          demo programs embedded in the binary
assets/fonts        Share Tech Mono (OFL), ProggyClean (MIT), Unscii-16 (public domain)
```

License: MIT OR Apache-2.0.
