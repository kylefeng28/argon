//! Documentation for every API function. The editor menus, Catalog, HELP,
//! completion and "did you mean" all read from this table (NFR-9).

use argon_core::api::{ApiEntry, ApiTable, Category};

macro_rules! entry {
    ($name:literal, $cat:ident, $sig:literal, $doc:literal, $ex:literal, $snip:literal) => {
        ApiEntry { name: $name, category: Category::$cat, signature: $sig, doc: $doc, example: $ex, snippet: $snip }
    };
}

pub static API_DOCS: &[ApiEntry] = &[
    // ---- System ----
    entry!("flip", System, "flip()", "Present the frame, read input and wait for the next frame. Call once per loop iteration.", "while true do cls() fill_circle(320, 200, 50) flip() end", "flip()"),
    entry!("cls", System, "cls([color])", "Clear the screen (and the text layer) to a color, default black.", "cls(rgb(20, 20, 40))", "cls(${color})"),
    entry!("wait", System, "wait(seconds)", "Keep presenting frames for the given time.", "print('Ready?') wait(1)", "wait(${seconds})"),
    entry!("time", System, "time()", "Seconds since the program started.", "local t = time()", "time()"),
    entry!("delta_time", System, "delta_time()", "Seconds the last frame took.", "x = x + 100 * delta_time()", "delta_time()"),
    entry!("frame", System, "frame()", "Number of frames presented so far.", "if frame() % 30 == 0 then beep(440, 0.05) end", "frame()"),
    entry!("fps", System, "fps([n])", "Get or set the target frame rate.", "fps(30)", "fps(${n})"),
    entry!("screen_size", System, "screen_size()", "Width and height of the virtual screen in pixels.", "local w, h = screen_size()", "screen_size()"),
    entry!("set_resolution", System, "set_resolution(w, h) or set_resolution(preset)", "Change the virtual resolution. Presets: tiny 128x128, retro 320x240, classic 640x400, hd 1280x800 (default), uhd 2560x1600, native (the window).", "set_resolution('retro')", "set_resolution(${w}, ${h})"),
    entry!("alt_screen", System, "alt_screen()", "Switch to a fresh, empty screen. The shell's screen is saved and comes back when the program ends (or calls main_screen).", "alt_screen() print('full screen program') wait_key()", "alt_screen()"),
    entry!("main_screen", System, "main_screen()", "Return to the screen that was showing before alt_screen().", "main_screen() print('back in the shell screen')", "main_screen()"),
    entry!("exit", System, "exit()", "End the program and return to the shell.", "if button_pressed('start') then exit() end", "exit()"),
    entry!("rgb", System, "rgb(r, g, b [, a])", "Make a color from 0-255 components.", "local orange = rgb(255, 128, 0)", "rgb(${r}, ${g}, ${b})"),
    entry!("hsv", System, "hsv(h, s, v [, a])", "Make a color from hue (degrees), saturation and value (0-1).", "fill_rect(0, 0, 10, 10, hsv(time() * 60, 1, 1))", "hsv(${h}, ${s}, ${v})"),
    entry!("color", System, "color([c])", "Get or set the default drawing color used when a color argument is omitted.", "color(rgb(255, 0, 0)) line(0, 0, 100, 100)", "color(${c})"),
    // ---- Math ----
    entry!("random", Math, "random([a [, b]])", "Random number: random() in [0,1), random(n) in 1..n, random(a, b) in a..b. Seedable.", "local x = random(1, 640)", "random(${a}, ${b})"),
    entry!("seed", Math, "seed(n)", "Seed the random number generator so runs are reproducible.", "seed(1234)", "seed(${n})"),
    entry!("clamp", Math, "clamp(x, lo, hi)", "Limit x to the range lo..hi.", "x = clamp(x, 0, 639)", "clamp(${x}, ${lo}, ${hi})"),
    entry!("lerp", Math, "lerp(a, b, t)", "Linear interpolation between a and b by t (0-1).", "x = lerp(x, target, 0.1)", "lerp(${a}, ${b}, ${t})"),
    entry!("round", Math, "round(x)", "Round to the nearest integer.", "local n = round(2.6)", "round(${x})"),
    entry!("sign", Math, "sign(x)", "-1, 0 or 1.", "dx = sign(target - x)", "sign(${x})"),
    entry!("dist", Math, "dist(x1, y1, x2, y2)", "Distance between two points.", "if dist(px, py, ex, ey) < 8 then hit() end", "dist(${x1}, ${y1}, ${x2}, ${y2})"),
    entry!("angle", Math, "angle(x1, y1, x2, y2)", "Angle in radians from point 1 to point 2.", "local a = angle(x, y, mx, my)", "angle(${x1}, ${y1}, ${x2}, ${y2})"),
    // ---- Text ----
    entry!("print", Text, "print(...)", "Print values at the text cursor, then a newline. Scrolls at the bottom.", "print('Score:', score)", "print(${text})"),
    entry!("write", Text, "write(...)", "Like print but without the newline.", "write('> ')", "write(${text})"),
    entry!("locate", Text, "locate(col, row)", "Move the text cursor (0-based).", "locate(0, 0) print('top left')", "locate(${col}, ${row})"),
    entry!("text_color", Text, "text_color(fg [, bg])", "Set colors for print/write. Omit bg for a transparent background.", "text_color(rgb(255, 255, 0), rgb(0, 0, 128))", "text_color(${fg})"),
    entry!("put_char", Text, "put_char(col, row, ch [, fg, bg])", "Write one character into a cell.", "put_char(10, 5, '@')", "put_char(${col}, ${row}, ${ch})"),
    entry!("get_char", Text, "get_char(col, row)", "Read a cell: character, fg, bg.", "local ch = get_char(10, 5)", "get_char(${col}, ${row})"),
    entry!("text_size", Text, "text_size()", "Columns and rows of the text grid.", "local cols, rows = text_size()", "text_size()"),
    entry!("cursor", Text, "cursor(visible)", "Show or hide the blinking text cursor.", "cursor(true)", "cursor(${visible})"),
    entry!("clear_text", Text, "clear_text()", "Clear only the text layer, leaving pixels.", "clear_text()", "clear_text()"),
    // ---- Input ----
    entry!("input", Input, "input([prompt])", "Read a line of text from the keyboard. Blocks until Enter.", "local name = input('Your name? ')", "input(${prompt})"),
    entry!("wait_key", Input, "wait_key()", "Wait for a key press and return its name.", "local k = wait_key()", "wait_key()"),
    entry!("read_key", Input, "read_key()", "Next typed key (character or name), or nil if none.", "local k = read_key() if k == 'q' then exit() end", "read_key()"),
    entry!("button", Input, "button(name [, player])", "Is a pad button held? Names: up down left right a b x y l r start select.", "if button('left') then x = x - 1 end", "button(${name})"),
    entry!("button_pressed", Input, "button_pressed(name [, player])", "Was the button pressed this frame?", "if button_pressed('a') then jump() end", "button_pressed(${name})"),
    entry!("button_released", Input, "button_released(name [, player])", "Was the button released this frame?", "if button_released('a') then stop_charge() end", "button_released(${name})"),
    entry!("key", Input, "key(name)", "Is a keyboard key held? e.g. 'a', 'space', 'enter', 'f1'.", "if key('space') then fire() end", "key(${name})"),
    entry!("key_pressed", Input, "key_pressed(name)", "Was the key pressed this frame?", "if key_pressed('p') then paused = not paused end", "key_pressed(${name})"),
    entry!("key_released", Input, "key_released(name)", "Was the key released this frame?", "if key_released('shift') then run = false end", "key_released(${name})"),
    entry!("mouse", Input, "mouse()", "Pointer position in screen pixels and whether it is over the screen: x, y, inside. No cursor is drawn; draw your own.", "local mx, my = mouse() fill_circle(mx, my, 4)", "mouse()"),
    entry!("mouse_button", Input, "mouse_button([button])", "Is a mouse button held? 'left' (default), 'right' or 'middle'.", "if mouse_button() then paint(mouse()) end", "mouse_button(${button})"),
    entry!("mouse_pressed", Input, "mouse_pressed([button])", "Was the mouse button pressed this frame?", "if mouse_pressed('right') then erase() end", "mouse_pressed(${button})"),
    entry!("mouse_released", Input, "mouse_released([button])", "Was the mouse button released this frame?", "if mouse_released() then drop() end", "mouse_released(${button})"),
    entry!("mouse_wheel", Input, "mouse_wheel()", "Wheel movement this frame in notches (positive = away from you); second value is horizontal.", "zoom = zoom * 1.1 ^ mouse_wheel()", "mouse_wheel()"),
    // ---- Draw ----
    entry!("pixel", Draw, "pixel(x, y [, color])", "Set one pixel.", "pixel(10, 10, rgb(255, 255, 255))", "pixel(${x}, ${y})"),
    entry!("line", Draw, "line(x1, y1, x2, y2 [, color])", "Draw a line.", "line(0, 0, 639, 399)", "line(${x1}, ${y1}, ${x2}, ${y2})"),
    entry!("line_width", Draw, "line_width(w)", "Thickness for subsequent lines (default 1).", "line_width(3)", "line_width(${w})"),
    entry!("rect", Draw, "rect(x, y, w, h [, color])", "Rectangle outline.", "rect(10, 10, 100, 50)", "rect(${x}, ${y}, ${w}, ${h})"),
    entry!("fill_rect", Draw, "fill_rect(x, y, w, h [, color])", "Filled rectangle.", "fill_rect(10, 10, 100, 50, rgb(0, 128, 255))", "fill_rect(${x}, ${y}, ${w}, ${h})"),
    entry!("circle", Draw, "circle(x, y, r [, color])", "Circle outline.", "circle(320, 200, 40)", "circle(${x}, ${y}, ${r})"),
    entry!("fill_circle", Draw, "fill_circle(x, y, r [, color])", "Filled circle.", "fill_circle(320, 200, 40, rgb(255, 200, 0))", "fill_circle(${x}, ${y}, ${r})"),
    entry!("ellipse", Draw, "ellipse(x, y, rx, ry [, color])", "Ellipse outline.", "ellipse(320, 200, 80, 40)", "ellipse(${x}, ${y}, ${rx}, ${ry})"),
    entry!("fill_ellipse", Draw, "fill_ellipse(x, y, rx, ry [, color])", "Filled ellipse.", "fill_ellipse(320, 200, 80, 40)", "fill_ellipse(${x}, ${y}, ${rx}, ${ry})"),
    entry!("triangle", Draw, "triangle(x1, y1, x2, y2, x3, y3 [, color])", "Triangle outline.", "triangle(10, 10, 50, 10, 30, 40)", "triangle(${x1}, ${y1}, ${x2}, ${y2}, ${x3}, ${y3})"),
    entry!("fill_triangle", Draw, "fill_triangle(x1, y1, x2, y2, x3, y3 [, color])", "Filled triangle.", "fill_triangle(10, 10, 50, 10, 30, 40)", "fill_triangle(${x1}, ${y1}, ${x2}, ${y2}, ${x3}, ${y3})"),
    entry!("polygon", Draw, "polygon({x1, y1, x2, y2, ...} [, color])", "Polygon outline from a flat list of coordinates.", "polygon({10, 10, 50, 10, 60, 40, 20, 50})", "polygon({${points}})"),
    entry!("fill_polygon", Draw, "fill_polygon({x1, y1, x2, y2, ...} [, color])", "Filled polygon from a flat list of coordinates.", "fill_polygon({10, 10, 50, 10, 60, 40, 20, 50})", "fill_polygon({${points}})"),
    entry!("draw_text", Draw, "draw_text(x, y, text [, color [, scale]])", "Draw text in pixels (not the text grid) using the console font. scale is a multiple of the 8x16 glyph; the default matches the text grid (2 at hd).", "draw_text(8, 8, 'SCORE ' .. score, rgb(255, 255, 255), 2)", "draw_text(${x}, ${y}, ${text})"),
    entry!("clip", Draw, "clip(x, y, w, h) or clip()", "Restrict drawing to a rectangle; clip() removes the restriction.", "clip(0, 0, 320, 200)", "clip(${x}, ${y}, ${w}, ${h})"),
    entry!("camera", Draw, "camera(dx, dy) or camera()", "Offset all subsequent drawing; camera() resets.", "camera(-player_x + 320, 0)", "camera(${dx}, ${dy})"),
    entry!("blend", Draw, "blend(mode)", "Blend mode for subsequent drawing: alpha, add, multiply, replace.", "blend('add')", "blend(${mode})"),
    // ---- Image ----
    entry!("load_image", Image, "load_image(path)", "Load a PNG from the program folder.", "local sheet = load_image('sprites.png')", "load_image(${path})"),
    entry!("new_image", Image, "new_image(w, h [, color])", "Create a blank image. Can be drawn into with set_target.", "local buf = new_image(128, 128)", "new_image(${w}, ${h})"),
    entry!("image_size", Image, "image_size(img)", "Width and height of an image.", "local w, h = image_size(sheet)", "image_size(${img})"),
    entry!("draw_image", Image, "draw_image(img, x, y [, opts])", "Draw a whole image. opts: scale, scale_x, scale_y, rotation, flip_x, flip_y, tint, alpha, origin_x, origin_y.", "draw_image(logo, 100, 50, { scale = 2 })", "draw_image(${img}, ${x}, ${y})"),
    entry!("draw_sprite", Image, "draw_sprite(img, sx, sy, sw, sh, x, y [, opts])", "Draw a rectangle of an image (a sprite from a sheet).", "draw_sprite(sheet, 0, 0, 16, 16, px, py, { flip_x = facing_left })", "draw_sprite(${img}, ${sx}, ${sy}, ${sw}, ${sh}, ${x}, ${y})"),
    entry!("draw_image_quad", Image, "draw_image_quad(img, x1, y1, x2, y2, x3, y3, x4, y4)", "Draw an image stretched onto four corners.", "draw_image_quad(img, 0, 0, 100, 10, 90, 90, 10, 100)", "draw_image_quad(${img}, ${x1}, ${y1}, ${x2}, ${y2}, ${x3}, ${y3}, ${x4}, ${y4})"),
    entry!("set_target", Image, "set_target(img) or set_target()", "Draw into an image instead of the screen; set_target() goes back to the screen.", "set_target(buf) cls() fill_circle(64, 64, 30) set_target()", "set_target(${img})"),
    // ---- GPU ----
    entry!("load_shader", Gpu, "load_shader(source or path)", "Compile a Shadertoy-style GLSL fragment shader: void mainImage(out vec4 fragColor, in vec2 fragCoord). Provides iResolution, iTime, iTimeDelta, iFrame, iMouse, iChannel0-3 and your own 'uniform float x;' declarations.", "local sh = load_shader(shader_source('plasma'))", "load_shader(${source})"),
    entry!("draw_shader", Gpu, "draw_shader(shader [, opts])", "Run a shader over the screen or opts.rect = {x, y, w, h}. opts.channels = {img, ...} feed iChannel0-3; opts.uniforms = {name = value} set user uniforms (numbers or {x, y[, z[, w]]} tables).", "draw_shader(sh, { uniforms = { speed = 2 } })", "draw_shader(${shader})"),
    entry!("set_shader", Gpu, "set_shader(shader [, opts]) or set_shader()", "Apply a shader to everything drawn afterwards (sprites, shapes, text). iChannel0 is the sprite's own texture and vUv/vColor are available. set_shader() turns it off.", "set_shader(tint, { uniforms = { color = {1, 0.5, 0.2} } }) draw_image(img, 0, 0) set_shader()", "set_shader(${shader})"),
    entry!("set_uniform", Gpu, "set_uniform(shader, name, value)", "Set a uniform's value that stays until changed; per-draw opts.uniforms override it.", "set_uniform(sh, 'speed', 0.5)", "set_uniform(${shader}, ${name}, ${value})"),
    entry!("shader_source", Gpu, "shader_source(name)", "GLSL source of a built-in shader to use or copy: plasma, ripple, pixelate, scanlines, tint.", "local sh = load_shader(shader_source('ripple'))", "shader_source(${name})"),
    // ---- Sound ----
    entry!("beep", Sound, "beep(freq, duration [, opts])", "Play a synth note. opts: wave (square, triangle, saw, sine, noise), volume 0-1, duty, slide (Hz/s), attack, release, pan.", "beep(440, 0.2) beep(880, 0.1, { wave = 'triangle' })", "beep(${freq}, ${duration})"),
    entry!("tone", Sound, "tone(channel, opts) or tone(channel)", "Start or change a held synth channel (1-8); tone(channel) stops it. Same opts as beep.", "tone(1, { freq = 220, wave = 'saw', volume = 0.4 })", "tone(${channel}, { freq = ${freq} })"),
    entry!("note_freq", Sound, "note_freq(name)", "Frequency in Hz of a note name like 'A4', 'C#5' or 'Eb3'.", "beep(note_freq('C5'), 0.25)", "note_freq(${name})"),
    entry!("load_sound", Sound, "load_sound(path)", "Load a WAV or OGG file from the program folder.", "local hit = load_sound('hit.wav')", "load_sound(${path})"),
    entry!("play_sound", Sound, "play_sound(snd [, opts])", "Play a loaded sound. opts: volume 0-1, pan -1..1, pitch (1 = normal), loop. Returns a voice handle.", "local v = play_sound(music, { loop = true, volume = 0.6 })", "play_sound(${snd})"),
    entry!("set_voice", Sound, "set_voice(voice, opts)", "Change volume, pan or pitch of a playing sound.", "set_voice(v, { pitch = 1.5 })", "set_voice(${voice}, { volume = ${volume} })"),
    entry!("stop_sound", Sound, "stop_sound(voice)", "Stop one playing sound.", "stop_sound(v)", "stop_sound(${voice})"),
    entry!("stop_all_sounds", Sound, "stop_all_sounds()", "Silence everything: sounds, tones and beeps.", "stop_all_sounds()", "stop_all_sounds()"),
    entry!("set_volume", Sound, "set_volume(v)", "Master volume 0-1.", "set_volume(0.5)", "set_volume(${v})"),
    // ---- File ----
    entry!("read_file", File, "read_file(path)", "Read a text file from the program folder; nil if missing.", "local data = read_file('level1.txt')", "read_file(${path})"),
    entry!("write_file", File, "write_file(path, text)", "Write a text file in the program folder.", "write_file('notes.txt', 'hello')", "write_file(${path}, ${text})"),
    entry!("append_file", File, "append_file(path, text)", "Append to a text file.", "append_file('log.txt', 'died\\n')", "append_file(${path}, ${text})"),
    entry!("file_exists", File, "file_exists(path)", "Does the file exist?", "if file_exists('save.txt') then load_game() end", "file_exists(${path})"),
    entry!("delete_file", File, "delete_file(path)", "Delete a file in the program folder.", "delete_file('temp.txt')", "delete_file(${path})"),
    entry!("list_files", File, "list_files([dir])", "Names of files in the program folder (or a sub folder).", "for _, f in ipairs(list_files()) do print(f) end", "list_files()"),
    entry!("save", File, "save(key, value)", "Store a value (number, string, boolean or table) that survives between runs.", "save('highscore', score)", "save(${key}, ${value})"),
    entry!("load", File, "load(key [, default])", "Read a value stored with save.", "local best = load('highscore', 0)", "load(${key}, ${default})"),
    entry!("require", File, "require(name)", "Run name.lua from the program folder once and return its result.", "local enemies = require('enemies')", "require(${name})"),
];

/// Lua keywords and control-flow snippets for the editor's Control menu.
pub static CONTROL_SNIPPETS: &[(&str, &str)] = &[
    ("if", "if ${cond} then\n\t${body}\nend"),
    ("if else", "if ${cond} then\n\t${body}\nelse\n\t${other}\nend"),
    ("for", "for ${i} = ${1}, ${10} do\n\t${body}\nend"),
    ("for in", "for ${k}, ${v} in pairs(${table}) do\n\t${body}\nend"),
    ("while", "while ${cond} do\n\t${body}\nend"),
    ("repeat", "repeat\n\t${body}\nuntil ${cond}"),
    ("function", "function ${name}(${args})\n\t${body}\nend"),
    ("local", "local ${name} = ${value}"),
    ("return", "return ${value}"),
    ("break", "break"),
    ("main loop", "while true do\n\tcls()\n\t${body}\n\tflip()\nend"),
];

pub fn api_table() -> ApiTable {
    let mut t = ApiTable::new();
    for e in API_DOCS {
        t.add(e.clone());
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_duplicate_names_and_all_have_docs() {
        let mut seen = std::collections::HashSet::new();
        for e in API_DOCS {
            assert!(seen.insert(e.name), "duplicate {}", e.name);
            assert!(!e.doc.is_empty() && !e.example.is_empty() && !e.snippet.is_empty(), "{}", e.name);
            assert!(e.signature.starts_with(e.name), "{} signature", e.name);
        }
    }
}
