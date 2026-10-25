# Argon - Lua API Sketch

## Conventions
- Naming rules:
  - Explicit snake_case verbs (e.g. `fill_circle`, `draw_sprite`)
  - `*_pressed` for edge events
  - `load_*` for resources
  - `set_*` for state
- Optional arguments are in brackets
- Colors are either a palette index (palette carts) or a packed value from `rgb()`/`hsv()` (truecolor carts)
- Coordinates are pixels in the virtual resolution, origin top-left, y down.

Status: Implemented: System (plus `alt_screen`/`main_screen`), Math,
Text, Input, Draw, Image (except `get_pixel`, `save_image`,
`image_pixel`/`image_upload`), Sound (plus `note_freq`; `play_notes` not yet)
and File. Not yet implemented: palette mode (`set_palette`/`palette_mode`),
the GPU section, `mouse()`. `argon docs` prints the live table; this file is
the design sketch.

## System

| Function | Description |
|----------|-------------|
| `flip()` | Present the frame, process input. The heart of every program. |
| `cls([color])` | Clear framebuffer (to `color`, default black) and the text layer; reset text cursor to 0,0. |
| `wait(seconds)` | Flip repeatedly for `seconds`. |
| `time()` | Seconds since program start (float). |
| `delta_time()` | Seconds elapsed during the last `flip()`. |
| `frame()` | Frames presented since start (integer). |
| `fps([n])` | Get or set the target frame rate. |
| `set_resolution(w, h)` or `set_resolution("retro")` | Change virtual resolution at runtime; clears the screen. |
| `screen_size()` | `w, h` of the virtual display. |
| `set_palette({...})` / `palette_mode(n)` | Define palette colors; switch color mode. |
| `exit()` | End the program and return to the shell. |
| `random([a [, b]])` | Seedable PRNG: `random()` in [0,1), `random(n)` in 1..n, `random(a,b)` in a..b. |
| `seed(n)` | Seed the PRNG. |
| `clamp(x, lo, hi)`, `lerp(a, b, t)`, `round(x)`, `sign(x)`, `dist(x1,y1,x2,y2)`, `angle(x1,y1,x2,y2)` | Math helpers in addition to the standard `math` library. |

## I/O - Text

| Function | Description |
|----------|-------------|
| `print(...)` | Write values separated by tabs at the text cursor, newline, scroll if needed. |
| `write(...)` | Same without the newline. |
| `locate(col, row)` | Move the text cursor (0-based). |
| `text_color(fg [, bg])` | Set colors for subsequent `print`/`write`. `bg = nil` is transparent. |
| `put_char(col, row, ch [, fg, bg])` | Write one character cell directly. |
| `get_char(col, row)` | Read a cell: `ch, fg, bg`. |
| `text_size()` | `cols, rows` of the text grid. |
| `cursor(visible)` | Show or hide the blinking text cursor. |
| `clear_text()` | Clear only the text layer. |

## I/O - Input

| Function | Description |
|----------|-------------|
| `input([prompt])` | Blocking line input with editing; returns a string (nil on Esc is not possible, Esc breaks the program). |
| `wait_key()` | Block until a key; returns its name. |
| `read_key()` | Pop the next key from the typed queue or nil. Returns printable characters as themselves and specials as names (`"enter"`, `"backspace"`, `"up"`). |
| `button(name [, player])` | Pad button held: `up down left right a b x y l r start select`. |
| `button_pressed(name [, player])` / `button_released(name [, player])` | Edge events for this frame. |
| `key(name)` / `key_pressed(name)` / `key_released(name)` | Raw keys: `"a"`.."z"`, `"0"`.."9"`, `"space"`, `"enter"`, `"shift"`, `"ctrl"`, `"f1"`.., `"left"`, etc. |
| `mouse()` | Reserved. Returns `0, 0, false, false, false` in api 1. |

## Draw

| Function | Description |
|----------|-------------|
| `rgb(r, g, b [, a])`, `hsv(h, s, v [, a])` | Build a truecolor value (components 0..255, h in degrees). |
| `color([c])` | Get or set the default draw color used when `color` is omitted. |
| `pixel(x, y [, c])` | Set one pixel. |
| `get_pixel(x, y)` | Read one pixel (slow path, cached per frame). |
| `line(x1, y1, x2, y2 [, c])` | 1-px line. `line_width(w)` sets thickness for subsequent lines. |
| `rect(x, y, w, h [, c])` / `fill_rect(x, y, w, h [, c])` | Outline / filled rectangle. |
| `circle(x, y, r [, c])` / `fill_circle(x, y, r [, c])` | |
| `ellipse(x, y, rx, ry [, c])` / `fill_ellipse(...)` | |
| `triangle(x1,y1,x2,y2,x3,y3 [, c])` / `fill_triangle(...)` | |
| `polygon({x1,y1,...} [, c])` / `fill_polygon({...} [, c])` | Convex or simple polygons. |
| `draw_text(x, y, str [, c [, scale]])` | Pixel text with the console font onto the framebuffer. |
| `clip(x, y, w, h)` / `clip()` | Set or clear the scissor rectangle. |
| `camera(dx, dy)` / `camera()` | Offset applied to all subsequent draws. |
| `blend(mode)` | `"alpha"` (default), `"add"`, `"multiply"`, `"replace"`. |

## Draw - Images

| Function | Description |
|----------|-------------|
| `load_image(path)` | PNG from the path -> `Image`. |
| `new_image(w, h [, color])` | Blank image (usable as a render target). |
| `image_size(img)` | `w, h`. |
| `draw_image(img, x, y [, opts])` | Draw whole image. `opts`: `scale`, `scale_x/scale_y`, `rotation` (radians), `flip_x`, `flip_y`, `tint`, `alpha`, `origin_x/origin_y`. |
| `draw_sprite(img, sx, sy, sw, sh, x, y [, opts])` | Draw a sub-rectangle with the same options. |
| `draw_image_quad(img, x1,y1,x2,y2,x3,y3,x4,y4 [, uvs])` | Arbitrary textured quad (pseudo-3D, distortion). |
| `set_target(img)` / `set_target()` | Redirect drawing into an image / back to the screen. |
| `save_image(img, path)` | Write PNG to the path. |
| `image_pixel(img, x, y [, c])` | CPU-side pixel set on an image that has not been uploaded yet; cheap way to build procedural textures. `image_upload(img)` commits. |

## GPU

| Function | Description |
|----------|-------------|
| `load_shader(src)` | Compile a Shadertoy-style GLSL fragment shader (string or `"file.glsl"`) -> `Shader`. Errors report the user's line numbers. |
| `draw_shader(sh [, opts])` | Run the shader over the full screen or `opts.rect = {x,y,w,h}`. `opts.channels = {img0, img1, ...}` bind `iChannel0..3`; `opts.uniforms = {name = value}` sets user uniforms (numbers, `{x,y}`, `{x,y,z}`, `{x,y,z,w}`, colors). |
| `set_shader(sh)` / `set_shader()` | Apply the shader to subsequent primitive and sprite draws (the sprite texture is `iChannel0`, vertex color is `vColor`). |
| `set_uniform(sh, name, value)` | Persistent uniform value on a shader. |
| `shader_source(name)` | Returns the source of a built-in shader (`"crt"`, `"blur"`, `"outline"`, `"palette_swap"`) to copy and modify. |

Provided uniforms (Shadertoy names): `iResolution`, `iTime`, `iTimeDelta`,
`iFrame`, `iMouse` (zeros in api 1), `iDate` (zeros), `iChannel0..3`,
`iChannelResolution[4]`.

Entry point: `void mainImage(out vec4 fragColor, in vec2 fragCoord)`.

## Sound

| Function | Description |
|----------|-------------|
| `load_sound(path)` | WAV or OGG -> `Sound`. |
| `play_sound(snd [, opts])` | `opts`: `volume` 0..1, `pan` -1..1, `pitch` multiplier, `loop`. Returns a `Voice` handle. |
| `stop_sound(voice)` / `stop_all_sounds()` | |
| `set_voice(voice, opts)` | Change volume/pan/pitch of a playing voice. |
| `beep(freq, duration [, opts])` | Fire-and-forget synth note. `opts`: `wave` (`"square"`, `"triangle"`, `"saw"`, `"sine"`, `"noise"`), `volume`, `duty`, `slide` (Hz/s), `attack`, `release`. |
| `tone(channel, opts)` | Start/modify a held synth channel 1..8 (`freq`, `wave`, `volume`, ...); `tone(channel)` stops it. |
| `set_volume(v)` | Master volume. |
| `play_notes(str [, opts])` | P1. `"C4 E4 G4 . C5"` at `opts.bpm`, returns when done unless `opts.async`. |

## Files

All paths are relative to the cart folder (or `SAVE/<cart>/` for `save`/`load`).

| Function | Description |
|----------|-------------|
| `read_file(path)` | Returns the contents as a string or nil + error. |
| `write_file(path, str)` / `append_file(path, str)` | |
| `file_exists(path)`, `delete_file(path)`, `list_files([dir])` | |
| `save(key, value)` / `load(key [, default])` | Per-cart key-value store; values are Lua tables, numbers, strings, booleans serialized as JSON. |
| `require(name)` | Load `name.lua` from the cart folder (cached). |

## Minimal programs

Hello:

```lua
print("Hello, world!")
wait_key()
```

Bouncing ball (linear model):

```lua
local x, y, dx, dy = 100, 50, 2, 1.5
while true do
  cls(rgb(10, 10, 30))
  fill_circle(x, y, 8, rgb(255, 200, 0))
  x, y = x + dx, y + dy
  if x < 8 or x > 632 then dx = -dx end
  if y < 8 or y > 352 then dy = -dy end
  if button("a") then dx, dy = dx * 1.1, dy * 1.1 end
  flip()
end
```
