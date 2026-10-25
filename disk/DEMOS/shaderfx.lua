-- shaderfx: post-processing. The scene (bouncing sprites) is drawn into an
-- image, then shown through a shader. Z cycles the effect, X toggles a
-- per-sprite tint shader. The ripple effect follows the mouse.
alt_screen()
local w, h = screen_size()

local scene = new_image(w, h)
local sheet = new_image(64, 32)
set_target(sheet)
cls(rgb(0, 0, 0, 0))
fill_circle(16, 16, 14, rgb(0, 220, 255))
fill_rect(36, 4, 24, 24, rgb(255, 140, 40))
set_target()

local effects = {
  { name = "none", shader = nil },
  { name = "ripple (mouse)", shader = load_shader(shader_source("ripple")) },
  { name = "pixelate", shader = load_shader(shader_source("pixelate")), uniforms = { size = 6 } },
  { name = "scanlines", shader = load_shader(shader_source("scanlines")) },
  { name = "chroma + wobble", shader = load_shader([[
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;
    uv.x += sin(uv.y * 20.0 + iTime * 3.0) * 0.004;
    float r = texture(iChannel0, uv + vec2(0.004, 0.0)).r;
    float g = texture(iChannel0, uv).g;
    float b = texture(iChannel0, uv - vec2(0.004, 0.0)).b;
    fragColor = vec4(r, g, b, 1.0);
}]]) },
}
local tint = load_shader(shader_source("tint"))
local fx, use_tint = 2, false

local things = {}
for i = 1, 24 do
  things[i] = { x = random() * w, y = random() * h, vx = (random() - 0.5) * 4, vy = (random() - 0.5) * 4, s = 1 + random() * 2, spr = random(0, 1), a = random() * 6.28 }
end

while true do
  if button_pressed("a") then fx = fx % #effects + 1 end
  if button_pressed("b") then use_tint = not use_tint end

  -- Draw the scene into an image.
  set_target(scene)
  cls(rgb(6, 8, 24))
  for x = 0, w, 40 do line(x, 0, x, h, rgb(14, 30, 50)) end
  for y = 0, h, 40 do line(0, y, w, y, rgb(14, 30, 50)) end
  if use_tint then set_shader(tint, { uniforms = { color = { 0.5 + 0.5 * math.sin(time()), 1, 0.5 + 0.5 * math.cos(time()) } } }) end
  for _, t in ipairs(things) do
    t.x, t.y, t.a = t.x + t.vx, t.y + t.vy, t.a + 0.02
    if t.x < 0 or t.x > w then t.vx = -t.vx end
    if t.y < 0 or t.y > h then t.vy = -t.vy end
    draw_sprite(sheet, t.spr * 32, 0, 32, 32, t.x, t.y, { scale = t.s, rotation = t.a, origin_x = 16, origin_y = 16 })
  end
  set_shader()
  draw_text(8, 8, "SCENE", rgb(255, 255, 255), 2)
  set_target()

  -- Show it through the current effect.
  local e = effects[fx]
  if e.shader then
    draw_shader(e.shader, { channels = { scene }, uniforms = e.uniforms })
  else
    draw_image(scene, 0, 0)
  end
  local th = 16 * (h // 400) + 4
  fill_rect(0, h - th, w, th, rgb(0, 0, 0, 160))
  draw_text(4, h - th + 2, string.format("effect: %s   Z next   X sprite tint shader %s", e.name, use_tint and "ON" or "off"), rgb(150, 220, 255))
  flip()
end
