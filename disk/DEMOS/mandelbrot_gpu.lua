-- mandelbrot_gpu: the Mandelbrot set as a fragment shader at full resolution.
-- Drag to pan, wheel to zoom (toward the pointer), Z/X zoom, C palette, R reset.
alt_screen()
local w, h = screen_size()

local sh = load_shader([[
uniform vec2 center;
uniform float zoom;
uniform float iters;
uniform float palette;

vec3 grid_palette(float t) {
    // near-black blue -> cyan -> white -> orange rim
    if (t < 0.55) { float k = pow(t / 0.55, 1.5); return vec3(0.01 + 0.08 * k, 0.03 + 0.72 * k, 0.07 + 0.9 * k); }
    if (t < 0.85) { float k = (t - 0.55) / 0.3; return mix(vec3(0.09, 0.75, 0.97), vec3(1.0), k); }
    float k = (t - 0.85) / 0.15;
    return mix(vec3(1.0), vec3(1.0, 0.5, 0.1), k);
}

void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = (fragCoord - 0.5 * iResolution.xy) / iResolution.x;
    vec2 c = center + uv * (3.0 / zoom);
    vec2 z = vec2(0.0);
    float i = 0.0;
    float m = 0.0;
    for (int k = 0; k < 2048; k++) {
        if (float(k) >= iters) break;
        z = vec2(z.x * z.x - z.y * z.y, 2.0 * z.x * z.y) + c;
        m = dot(z, z);
        if (m > 256.0) break;
        i += 1.0;
    }
    if (i >= iters) { fragColor = vec4(0.0, 0.0, 0.0, 1.0); return; }
    // smooth iteration count
    float nu = log2(log2(m) * 0.5);
    float t = (i + 1.0 - nu) / iters;
    t = pow(clamp(t, 0.0, 1.0), 0.7);
    vec3 col;
    if (palette < 0.5) col = grid_palette(t);
    else if (palette < 1.5) col = 0.5 + 0.5 * cos(6.2831 * (t * 3.0 + vec3(0.0, 0.33, 0.67)) + iTime * 0.5);
    else col = vec3(sqrt(t));
    fragColor = vec4(col, 1.0);
}
]])

local cx, cy, zoom = -0.6, 0.0, 1.0
local pal = 0
local last_mx, last_my = 0, 0

-- Screen -> complex plane. The shader's fragCoord is y-up (Shadertoy
-- style) while mouse() is y-down, hence the flipped y here.
local function world(mx, my)
  return cx + (mx - w / 2) / w * (3 / zoom), cy + (h / 2 - my) / w * (3 / zoom)
end

while true do
  local mx, my = mouse()
  if mouse_pressed() then last_mx, last_my = mx, my end
  if mouse_button() then
    cx = cx - (mx - last_mx) / w * (3 / zoom)
    cy = cy + (my - last_my) / w * (3 / zoom)
    last_mx, last_my = mx, my
  end
  local wheel = mouse_wheel()
  if wheel ~= 0 then
    -- Zoom about the pointer so the point under it stays put.
    local wx, wy = world(mx, my)
    zoom = clamp(zoom * 1.25 ^ wheel, 0.5, 1e6)
    local nx, ny = world(mx, my)
    cx, cy = cx + (wx - nx), cy + (wy - ny)
  end
  if button("a") then zoom = math.min(zoom * 1.03, 1e6) end
  if button("b") then zoom = math.max(zoom / 1.03, 0.5) end
  if button_pressed("x") then pal = (pal + 1) % 3 end
  if key_pressed("r") then cx, cy, zoom = -0.6, 0.0, 1.0 end
  local iters = clamp(96 + math.log(zoom, 2) * 40, 96, 2000)

  draw_shader(sh, { uniforms = { center = { cx, cy }, zoom = zoom, iters = iters, palette = pal } })
  fill_rect(0, h - 18, w, 18, rgb(0, 0, 0, 160))
  draw_text(4, h - 16, string.format("%.8f %+.8fi  zoom %.0f  iter %d   drag/wheel  C palette  R reset", cx, cy, zoom, iters), rgb(150, 220, 255))
  flip()
end
