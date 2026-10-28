# Writing Argon programs (guide for code-generating models)

You are writing a program for Argon, a fantasy console. Output one complete
Lua (Luau) file and nothing else unless asked. Follow these rules exactly.

## The model

- The program runs top to bottom. There are no callbacks. To animate, use
  `while true do ... flip() end`. `flip()` shows the frame, reads input and
  waits for the next frame (60 fps by default). A program with no loop just
  runs and ends.
- The screen is 640x400 pixels, origin top-left, y down. Use
  `local w, h = screen_size()` instead of hard-coding when convenient.
- There is a text layer of 80x25 character cells on top of the pixels.
  `print`, `write`, `locate`, `put_char` use it. `draw_text(x, y, s)` draws text
  in pixels instead.
- Pixels persist between frames. Call `cls()` at the start of each frame for
  animation, or never call it for a drawing that accumulates.
- Start graphical programs with `alt_screen()` so the user's shell screen is
  restored when the program ends. You may follow it with
  `set_resolution("retro")` (320x240, faster for per-pixel work) or another
  preset; `screen_size()` then returns the new size.
- Esc always stops the program; you do not need to handle it. Call `exit()` to
  end early.
- Colors are numbers from `rgb(r, g, b)` (0-255) or `hsv(h, s, v)` (h in
  degrees, s and v 0-1). Every drawing function takes an optional trailing
  color; omitted means the current `color()`.
- Coordinates are floored to whole pixels. Fractional positions are fine.
- `random(a, b)` returns an integer in a..b; `random()` a float in [0, 1).
- Blocking helpers: `input(prompt)` reads a line, `wait_key()` waits for a key,
  `wait(seconds)` pauses while still showing frames. They call `flip()` for
  you.
- Lua is Luau. Available: `string`, `table`, `math`, `utf8`, `coroutine`,
  `tostring`, `tonumber`, `pairs`, `ipairs`, `pcall`, `string.format`.
  Not available: `io`, `os`, `require` of anything outside the program folder,
  `loadstring`, `dofile`, `print` to a terminal (print goes to the screen).
  Use `//` for integer division, `%` for modulo, `..` for concatenation,
  `#t` for table length; tables are 1-based.
- Keep programs self-contained: no external assets unless the user provides
  them. Build sprites with `new_image` + `set_target` + drawing calls instead
  of loading PNGs.
- Prefer small, readable programs with a short comment header. Do not define
  `update`/`draw` functions that nobody calls.

## Input names

- Pad: `button("up")`, `"down"`, `"left"`, `"right"`, `"a"` (Z key), `"b"` (X),
  `"x"` (A), `"y"` (S), `"l"`, `"r"`, `"start"` (Enter), `"select"` (Shift).
  `button_pressed(name)` is true only on the frame the button went down.
- Keys: `key("space")`, `key("a")`..`key("z")`, `"0"`..`"9"`, `"enter"`,
  `"escape"`, `"backspace"`, `"tab"`, `"up"`, `"down"`, `"left"`, `"right"`,
  `"shift"`, `"ctrl"`, `"f1"`..`"f12"`. `read_key()` returns the next typed
  character (e.g. `"a"`, `"A"`, `"1"`) or a key name, or nil.

## Mouse

- `local x, y, inside = mouse()` gives the pointer in screen pixels. Argon
  never draws a cursor: if the program uses the mouse, draw a crosshair at
  (x, y) yourself.
- `mouse_button([name])`, `mouse_pressed([name])`, `mouse_released([name])`
  take `"left"` (default), `"right"` or `"middle"`.
- `mouse_wheel()` returns the notches scrolled this frame (positive = away
  from the user), e.g. `zoom = zoom * 1.1 ^ mouse_wheel()`.

## Shaders (GPU)

- `local sh = load_shader(src)` compiles Shadertoy-style GLSL: the shader
  defines `void mainImage(out vec4 fragColor, in vec2 fragCoord)` and may use
  `iResolution`, `iTime`, `iTimeDelta`, `iFrame`, `iMouse`, `iChannel0..3`.
  Declare your own inputs as `uniform float speed;` (also vec2/vec3/vec4/
  int/bool) at the top of the shader.
- `draw_shader(sh, { uniforms = { speed = 2, center = {x, y} }, channels =
  { img }, rect = {x, y, w, h} })` runs it over the screen (or the rect).
  Channel images are sampled bottom-up like Shadertoy, so an image drawn
  with `texture(iChannel0, fragCoord / iResolution.xy)` appears upright.
- `set_shader(sh)` ... `set_shader()` applies a shader to sprites and shapes
  drawn in between; inside it `vUv` and `vColor` are the sprite's texture
  coordinate and tint, and `iChannel0` is its texture (sample with
  `textureTopDown(iChannel0, vUv)`).
- `shader_source("plasma" | "ripple" | "pixelate" | "scanlines" | "tint")`
  returns ready-made GLSL to load or copy.
- Fragment shaders cannot read the screen; draw the scene into an image with
  `set_target(img)` and pass it as a channel for post-processing.
- Mandelbrot/Julia/raymarching belong in a shader, not in Lua loops.

## Sound

- `beep(freq, duration, { wave = "square" })` plays one note; waves are
  `square`, `triangle`, `saw`, `sine`, `noise`. Options: `volume` 0-1, `duty`,
  `slide` (Hz per second, negative for a falling laser), `attack`, `release`,
  `pan`.
- `tone(channel, { freq = 440, wave = "triangle", volume = 0.5 })` holds a
  note on channel 1-8 until `tone(channel)` stops it. Use it for music.
- `note_freq("C#5")` converts a note name to Hz.
- Never leave a tone playing in a tight loop without reason; beeps are fire
  and forget.

## Function reference (signature -- meaning)

## System
flip()  -- Present the frame, read input and wait for the next frame. Call once per loop iteration.
cls([color])  -- Clear the screen (and the text layer) to a color, default black.
wait(seconds)  -- Keep presenting frames for the given time.
time()  -- Seconds since the program started.
delta_time()  -- Seconds the last frame took.
frame()  -- Number of frames presented so far.
fps([n])  -- Get or set the target frame rate.
screen_size()  -- Width and height of the virtual screen in pixels.
set_resolution(w, h) or set_resolution(preset)  -- Change the virtual resolution. Presets: tiny, retro, classic, hd.
alt_screen()  -- Switch to a fresh, empty screen. The shell's screen is saved and comes back when the program ends (or calls main_screen).
main_screen()  -- Return to the screen that was showing before alt_screen().
exit()  -- End the program and return to the shell.
rgb(r, g, b [, a])  -- Make a color from 0-255 components.
hsv(h, s, v [, a])  -- Make a color from hue (degrees), saturation and value (0-1).
color([c])  -- Get or set the default drawing color used when a color argument is omitted.

## Text
print(...)  -- Print values at the text cursor, then a newline. Scrolls at the bottom.
write(...)  -- Like print but without the newline.
locate(col, row)  -- Move the text cursor (0-based).
text_color(fg [, bg])  -- Set colors for print/write. Omit bg for a transparent background.
put_char(col, row, ch [, fg, bg])  -- Write one character into a cell.
get_char(col, row)  -- Read a cell: character, fg, bg.
text_size()  -- Columns and rows of the text grid.
cursor(visible)  -- Show or hide the blinking text cursor.
clear_text()  -- Clear only the text layer, leaving pixels.

## Input
input([prompt])  -- Read a line of text from the keyboard. Blocks until Enter.
wait_key()  -- Wait for a key press and return its name.
read_key()  -- Next typed key (character or name), or nil if none.
button(name [, player])  -- Is a pad button held? Names: up down left right a b x y l r start select.
button_pressed(name [, player])  -- Was the button pressed this frame?
button_released(name [, player])  -- Was the button released this frame?
key(name)  -- Is a keyboard key held? e.g. 'a', 'space', 'enter', 'f1'.
key_pressed(name)  -- Was the key pressed this frame?
key_released(name)  -- Was the key released this frame?

## Draw
pixel(x, y [, color])  -- Set one pixel.
line(x1, y1, x2, y2 [, color])  -- Draw a line.
line_width(w)  -- Thickness for subsequent lines (default 1).
rect(x, y, w, h [, color])  -- Rectangle outline.
fill_rect(x, y, w, h [, color])  -- Filled rectangle.
circle(x, y, r [, color])  -- Circle outline.
fill_circle(x, y, r [, color])  -- Filled circle.
ellipse(x, y, rx, ry [, color])  -- Ellipse outline.
fill_ellipse(x, y, rx, ry [, color])  -- Filled ellipse.
triangle(x1, y1, x2, y2, x3, y3 [, color])  -- Triangle outline.
fill_triangle(x1, y1, x2, y2, x3, y3 [, color])  -- Filled triangle.
polygon({x1, y1, x2, y2, ...} [, color])  -- Polygon outline from a flat list of coordinates.
fill_polygon({x1, y1, x2, y2, ...} [, color])  -- Filled polygon from a flat list of coordinates.
draw_text(x, y, text [, color [, scale]])  -- Draw text in pixels (not the text grid) using the console font.
clip(x, y, w, h) or clip()  -- Restrict drawing to a rectangle; clip() removes the restriction.
camera(dx, dy) or camera()  -- Offset all subsequent drawing; camera() resets.
blend(mode)  -- Blend mode for subsequent drawing: alpha, add, multiply, replace.

## Image
load_image(path)  -- Load a PNG from the program folder.
new_image(w, h [, color])  -- Create a blank image. Can be drawn into with set_target.
image_size(img)  -- Width and height of an image.
draw_image(img, x, y [, opts])  -- Draw a whole image. opts: scale, scale_x, scale_y, rotation, flip_x, flip_y, tint, alpha, origin_x, origin_y.
draw_sprite(img, sx, sy, sw, sh, x, y [, opts])  -- Draw a rectangle of an image (a sprite from a sheet).
draw_image_quad(img, x1, y1, x2, y2, x3, y3, x4, y4)  -- Draw an image stretched onto four corners.
set_target(img) or set_target()  -- Draw into an image instead of the screen; set_target() goes back to the screen.

## Mouse

- `local x, y, inside = mouse()` gives the pointer in screen pixels. Argon
  never draws a cursor: if the program uses the mouse, draw a crosshair at
  (x, y) yourself.
- `mouse_button([name])`, `mouse_pressed([name])`, `mouse_released([name])`
  take `"left"` (default), `"right"` or `"middle"`.
- `mouse_wheel()` returns the notches scrolled this frame (positive = away
  from the user), e.g. `zoom = zoom * 1.1 ^ mouse_wheel()`.

## Shaders (GPU)

- `local sh = load_shader(src)` compiles Shadertoy-style GLSL: the shader
  defines `void mainImage(out vec4 fragColor, in vec2 fragCoord)` and may use
  `iResolution`, `iTime`, `iTimeDelta`, `iFrame`, `iMouse`, `iChannel0..3`.
  Declare your own inputs as `uniform float speed;` (also vec2/vec3/vec4/
  int/bool) at the top of the shader.
- `draw_shader(sh, { uniforms = { speed = 2, center = {x, y} }, channels =
  { img }, rect = {x, y, w, h} })` runs it over the screen (or the rect).
  Channel images are sampled bottom-up like Shadertoy, so an image drawn
  with `texture(iChannel0, fragCoord / iResolution.xy)` appears upright.
- `set_shader(sh)` ... `set_shader()` applies a shader to sprites and shapes
  drawn in between; inside it `vUv` and `vColor` are the sprite's texture
  coordinate and tint, and `iChannel0` is its texture (sample with
  `textureTopDown(iChannel0, vUv)`).
- `shader_source("plasma" | "ripple" | "pixelate" | "scanlines" | "tint")`
  returns ready-made GLSL to load or copy.
- Fragment shaders cannot read the screen; draw the scene into an image with
  `set_target(img)` and pass it as a channel for post-processing.
- Mandelbrot/Julia/raymarching belong in a shader, not in Lua loops.

## Sound
beep(freq, duration [, opts])  -- Play a synth note. opts: wave (square, triangle, saw, sine, noise), volume 0-1, duty, slide (Hz/s), attack, release, pan.
tone(channel, opts) or tone(channel)  -- Start or change a held synth channel (1-8); tone(channel) stops it. Same opts as beep.
note_freq(name)  -- Frequency in Hz of a note name like 'A4', 'C#5' or 'Eb3'.
load_sound(path)  -- Load a WAV or OGG file from the program folder.
play_sound(snd [, opts])  -- Play a loaded sound. opts: volume 0-1, pan -1..1, pitch (1 = normal), loop. Returns a voice handle.
set_voice(voice, opts)  -- Change volume, pan or pitch of a playing sound.
stop_sound(voice)  -- Stop one playing sound.
stop_all_sounds()  -- Silence everything: sounds, tones and beeps.
set_volume(v)  -- Master volume 0-1.

## File
read_file(path)  -- Read a text file from the program folder; nil if missing.
write_file(path, text)  -- Write a text file in the program folder.
append_file(path, text)  -- Append to a text file.
file_exists(path)  -- Does the file exist?
delete_file(path)  -- Delete a file in the program folder.
list_files([dir])  -- Names of files in the program folder (or a sub folder).
save(key, value)  -- Store a value (number, string, boolean or table) that survives between runs.
load(key [, default])  -- Read a value stored with save.
require(name)  -- Run name.lua from the program folder once and return its result.

## Math
random([a [, b]])  -- Random number: random() in [0,1), random(n) in 1..n, random(a, b) in a..b. Seedable.
seed(n)  -- Seed the random number generator so runs are reproducible.
clamp(x, lo, hi)  -- Limit x to the range lo..hi.
lerp(a, b, t)  -- Linear interpolation between a and b by t (0-1).
round(x)  -- Round to the nearest integer.
sign(x)  -- -1, 0 or 1.
dist(x1, y1, x2, y2)  -- Distance between two points.
angle(x1, y1, x2, y2)  -- Angle in radians from point 1 to point 2.

(Regenerate this list with `argon docs --compact`.)

## Complete examples

A text program:

```lua
-- guess: a number guessing game
cls()
print("I picked a number from 1 to 100.")
local secret = random(1, 100)
local tries = 0
while true do
  local guess = tonumber(input("Your guess? "))
  if guess == nil then
    print("Please type a number.")
  else
    tries = tries + 1
    if guess < secret then print("Higher.")
    elseif guess > secret then print("Lower.")
    else
      print(string.format("Yes! %d in %d tries.", secret, tries))
      break
    end
  end
end
beep(880, 0.2)
print("Press any key.")
wait_key()
```

An animated program with input and sound:

```lua
-- dodge: move with arrows, avoid the falling blocks
alt_screen()
local w, h = screen_size()
local px, py = w / 2, h - 30
local blocks = {}
local score, alive = 0, true

while alive do
  cls(rgb(15, 15, 35))
  if button("left") then px = px - 4 end
  if button("right") then px = px + 4 end
  px = clamp(px, 10, w - 10)

  if frame() % 20 == 0 then
    blocks[#blocks + 1] = { x = random(0, w - 20), y = -20, s = 2 + score / 10 }
  end
  for i = #blocks, 1, -1 do
    local b = blocks[i]
    b.y = b.y + b.s
    if b.y > h then
      table.remove(blocks, i)
      score = score + 1
      beep(600, 0.03, { wave = "triangle", volume = 0.3 })
    elseif px + 8 > b.x and px - 8 < b.x + 20 and py + 8 > b.y and py - 8 < b.y + 20 then
      alive = false
    end
    fill_rect(b.x, b.y, 20, 20, hsv(b.y, 0.8, 1))
  end
  fill_circle(px, py, 8, rgb(255, 230, 120))
  draw_text(8, 8, "SCORE " .. score, rgb(255, 255, 255), 2)
  flip()
end

beep(200, 0.4, { wave = "saw", slide = -300 })
draw_text(w / 2 - 72, h / 2 - 8, "GAME OVER", rgb(255, 80, 80), 2)
flip()
wait(0.5)
wait_key()
```

A drawing that accumulates (no cls in the loop):

```lua
-- spiral: draws forever, Esc to stop
alt_screen()
cls(rgb(0, 0, 0))
local w, h = screen_size()
local a, r = 0, 0
while true do
  for _ = 1, 20 do
    a = a + 0.05
    r = r + 0.03
    pixel(w / 2 + math.cos(a) * r, h / 2 + math.sin(a) * r, hsv(a * 20, 1, 1))
    if r > h / 2 then r = 0 end
  end
  flip()
end
```
