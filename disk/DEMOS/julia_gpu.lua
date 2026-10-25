-- julia_gpu: Julia sets on the GPU. The mouse position is c; hold the
-- left button to freeze it. Wheel zooms, Z toggles the automatic orbit.
alt_screen()
local w, h = screen_size()

local sh = load_shader([[
uniform vec2 c;
uniform float zoom;
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 z = (fragCoord - 0.5 * iResolution.xy) / iResolution.y * (2.6 / zoom);
    float i = 0.0;
    float m = 0.0;
    for (int k = 0; k < 300; k++) {
        z = vec2(z.x * z.x - z.y * z.y, 2.0 * z.x * z.y) + c;
        m = dot(z, z);
        if (m > 64.0) break;
        i += 1.0;
    }
    if (i >= 300.0) { fragColor = vec4(0.015, 0.02, 0.08, 1.0); return; }
    float t = (i - log2(log2(m) * 0.5)) / 300.0;
    t = pow(clamp(t, 0.0, 1.0), 0.45);
    // Electric: deep blue, cyan, white, with a magenta rim near the set.
    vec3 col = mix(vec3(0.02, 0.05, 0.15), vec3(0.0, 0.8, 1.0), smoothstep(0.0, 0.6, t));
    col = mix(col, vec3(1.0), smoothstep(0.6, 0.9, t));
    col = mix(col, vec3(0.9, 0.3, 1.0), smoothstep(0.9, 1.0, t));
    fragColor = vec4(col, 1.0);
}
]])

local auto, t = true, 2.6
local cr, ci = 0, 0
local zoom = 1

while true do
  local mx, my, inside = mouse()
  if button_pressed("a") then auto = not auto end
  zoom = clamp(zoom * 1.2 ^ mouse_wheel(), 0.5, 50)
  if inside and not auto then
    if not mouse_button() then
      cr, ci = (mx / w - 0.5) * 3.0, (0.5 - my / h) * 2.0
    end
  else
    t = t + delta_time() * 0.15
    cr, ci = 0.7885 * math.cos(t), 0.7885 * math.sin(t)
  end
  draw_shader(sh, { uniforms = { c = { cr, ci }, zoom = zoom } })
  local th = 16 * (h // 400) + 4
  fill_rect(0, h - th, w, th, rgb(0, 0, 0, 160))
  draw_text(4, h - th + 2, string.format("c = %.4f %+.4fi   %s   wheel zoom", cr, ci, auto and "Z: mouse control" or "move mouse, hold to freeze, Z: orbit"), rgb(150, 220, 255))
  flip()
end
