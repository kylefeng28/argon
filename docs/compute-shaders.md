Tier 4 GPU (FR-G12). Programs can now simulate millions of elements on the
GPU without the data ever reaching Lua.

Lua API:
- load_compute(src): GLSL compute kernel compiled by naga (same globals as
  fragment shaders, iCount = dispatch size, user uniforms).
- dispatch(cs, n | {x, y[, z]}, {buffers, uniforms}): runs in order with the
  frame's drawing (encoded between render passes).
- new_buffer(n, float|vec2|vec4|int|uint), write_buffer(buf, numbers [, start]).
- draw_points(buf, {colors, color, size, count}): one square per element,
  read straight from the storage buffer (points.wgsl).
- draw_shader opts.buffers: fragment shaders read buffers (readonly).

Implementation:
- Buffers live in bind group 1; the preprocessor adds set = 1 to
  `layout(...) buffer` declarations. Reflection rejects other sets and
  bindings >= 8; a writable buffer bound twice is an error.
- Request the adapter's limits instead of downlevel defaults.
- Release builds log uncaptured GPU errors instead of panicking.
- Renderer waits for in-flight GPU work on drop, and the headless loop
  waits per frame (nothing presents, so nothing throttled submission and
  wgpu-core timed out on shutdown with heavy frames on SwiftShader).
- Uniform declarations may carry a trailing // comment.

Demos: particles_gpu (1M particles in a flow field, mouse attract/repel)
and reaction_diffusion (Gray-Scott on ping-pong buffers).

Tests: 4 GPU tests (dispatch + uniforms + iCount, ping-pong ordering,
draw_points after dispatch in one frame, per-point colors and a fragment
shader reading a buffer). Docs: manual, agent guide (compute section and
regenerated reference), README, requirements, API sketch.

Known gaps: atomics (naga's GLSL frontend lacks atomicAdd), storage
textures, Lua-side buffer readback. SwiftShader occasionally segfaults
inside its own worker threads with 1M points; Metal is clean.

