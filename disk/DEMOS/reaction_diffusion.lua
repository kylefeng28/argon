-- reaction_diffusion: the Gray-Scott model on the GPU. Two chemicals, U and
-- V, diffuse and react (U + 2V -> 3V); feed and kill rates decide whether
-- you get coral, cells dividing, mazes or worms.
-- Left mouse paints V, right erases. Wheel: brush size. 1-5: presets.
-- R: reseed. Up/Down: simulation speed.
alt_screen()
local w, h = screen_size()
local gw, gh = w // 2, h // 2          -- simulation grid (640x400 at hd)
local cells = gw * gh

-- Ping-pong buffers: each step reads one and writes the other.
local a = new_buffer(cells, "vec2")    -- (U, V) per cell
local b = new_buffer(cells, "vec2")

local init = load_compute([[
layout(local_size_x = 16, local_size_y = 16) in;
layout(std430, binding = 0) buffer G { vec2 g[]; };
uniform float salt;

float hash(uvec2 p) {
    uint x = p.x * 1664525u + p.y * 1013904223u + uint(salt) * 2654435761u;
    x ^= x >> 16; x *= 0x7feb352dU; x ^= x >> 15; x *= 0x846ca68bU; x ^= x >> 16;
    return float(x) / 4294967295.0;
}

void main() {
    uvec2 c = gl_GlobalInvocationID.xy;
    if (c.x >= iCount.x || c.y >= iCount.y) return;
    // A few random 10x10 seeds of V in a sea of U.
    float v = hash(c / 10u) > 0.985 ? 1.0 : 0.0;
    g[c.y * iCount.x + c.x] = vec2(1.0, v);
}
]])

local step = load_compute([[
layout(local_size_x = 16, local_size_y = 16) in;
layout(std430, binding = 0) readonly buffer Src { vec2 src[]; };
layout(std430, binding = 1) buffer Dst { vec2 dst[]; };
uniform float feed;
uniform float kill;
uniform vec2 brush;     // grid coordinates
uniform float radius;   // in cells
uniform float paint;    // +1 add V, -1 erase, 0 none

void main() {
    uvec2 c = gl_GlobalInvocationID.xy;
    uint W = iCount.x, H = iCount.y;
    if (c.x >= W || c.y >= H) return;
    // Wrap-around neighbours.
    uint xm = (c.x + W - 1u) % W, xp = (c.x + 1u) % W;
    uint ym = (c.y + H - 1u) % H, yp = (c.y + 1u) % H;
    vec2 m = src[c.y * W + c.x];
    // 3x3 Laplacian: center -1, edges 0.2, corners 0.05.
    vec2 lap = -m
        + 0.2 * (src[c.y * W + xm] + src[c.y * W + xp] + src[ym * W + c.x] + src[yp * W + c.x])
        + 0.05 * (src[ym * W + xm] + src[ym * W + xp] + src[yp * W + xm] + src[yp * W + xp]);
    float u = m.x, v = m.y;
    float uvv = u * v * v;
    u += 1.0 * lap.x - uvv + feed * (1.0 - u);
    v += 0.5 * lap.y + uvv - (feed + kill) * v;
    if (paint != 0.0 && distance(vec2(c), brush) < radius) {
        if (paint > 0.0) { v = 1.0; } else { u = 1.0; v = 0.0; }
    }
    dst[c.y * W + c.x] = clamp(vec2(u, v), 0.0, 1.0);
}
]])

local show = load_shader([[
layout(std430, binding = 0) buffer G { vec2 g[]; };
uniform vec2 grid;

float V(ivec2 c) {
    ivec2 n = ivec2(grid);
    c = clamp(c, ivec2(0), n - 1);
    return g[c.y * n.x + c.x].y;
}

void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    // fragCoord is y-up; the grid's row 0 is the top of the screen.
    vec2 uv = fragCoord / iResolution.xy;
    ivec2 c = ivec2(uv.x * grid.x, (1.0 - uv.y) * grid.y);
    float v = V(c);
    // Surface normal from the V field gives the pattern a lit, glassy look.
    vec2 grad = vec2(V(c + ivec2(1, 0)) - V(c - ivec2(1, 0)), V(c + ivec2(0, 1)) - V(c - ivec2(0, 1)));
    vec3 n = normalize(vec3(-grad * 6.0, 1.0));
    float light = clamp(dot(n, normalize(vec3(-0.5, -0.6, 0.8))), 0.0, 1.0);
    float t = smoothstep(0.08, 0.45, v);
    vec3 bg = vec3(0.01, 0.02, 0.06);
    vec3 col = mix(bg, vec3(0.0, 0.75, 1.0), t);
    col = mix(col, vec3(0.9, 1.0, 1.0), smoothstep(0.35, 0.6, v));
    col *= 0.55 + 0.6 * light;
    // Orange rim where the front is advancing.
    col += vec3(1.0, 0.45, 0.1) * smoothstep(0.08, 0.25, length(grad)) * (1.0 - t) * 0.8;
    fragColor = vec4(col, 1.0);
}
]])

local presets = {
  { "coral", 0.0545, 0.062 },
  { "mitosis", 0.0367, 0.0649 },
  { "maze", 0.029, 0.057 },
  { "worms", 0.078, 0.061 },
  { "u-skate", 0.062, 0.0609 },
}
local which = 1
local steps = 16
local radius = 6
local salt = 1

local function reseed()
  dispatch(init, { gw, gh }, { buffers = { a }, uniforms = { salt = salt } })
  salt = salt + 1
end
reseed()

while true do
  for k = 1, #presets do
    if key_pressed(tostring(k)) then which = k end
  end
  if key_pressed("r") then reseed() end
  if button_pressed("up") then steps = math.min(steps + 4, 64) end
  if button_pressed("down") then steps = math.max(steps - 4, 2) end
  radius = clamp(radius * 1.2 ^ mouse_wheel(), 2, 60)

  local mx, my, inside = mouse()
  local paint = 0
  if inside and mouse_button("left") then paint = 1 elseif inside and mouse_button("right") then paint = -1 end
  local p = presets[which]
  local u = { feed = p[2], kill = p[3], brush = { mx * gw / w, my * gh / h }, radius = radius, paint = paint }

  -- An even number of steps, so the newest state always ends up in `a`.
  for _ = 1, steps // 2 do
    dispatch(step, { gw, gh }, { buffers = { a, b }, uniforms = u })
    dispatch(step, { gw, gh }, { buffers = { b, a }, uniforms = u })
  end
  draw_shader(show, { buffers = { a }, uniforms = { grid = { gw, gh } } })

  if inside then circle(mx, my, radius * w / gw, rgb(255, 255, 255, 120)) end
  local ts = h // 400
  fill_rect(0, h - 18 * ts, w, 18 * ts, rgb(0, 0, 0, 170))
  draw_text(4, h - 17 * ts, string.format("%s  F %.4f k %.4f  %d steps/frame  %dx%d   1-5 preset  mouse paint/erase  wheel brush  R reseed",
    p[1], p[2], p[3], steps, gw, gh), rgb(150, 220, 255))
  flip()
end
