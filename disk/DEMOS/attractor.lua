-- attractor: strange attractors drawn as glowing point clouds that slowly
-- rotate in 3D. Z switches attractor, X toggles a hue sweep, arrows spin.
-- Mouse: drag to rotate, wheel to zoom.
alt_screen()
local w, h = screen_size()

local systems = {
  { name = "Lorenz", dt = 0.006, scale = 7, cz = -25,
    f = function(x, y, z) return 10 * (y - x), x * (28 - z) - y, x * y - 8 / 3 * z end },
  { name = "Rossler", dt = 0.03, scale = 12, cz = -8,
    f = function(x, y, z) return -y - z, x + 0.2 * y, 0.2 + z * (x - 5.7) end },
  { name = "Aizawa", dt = 0.012, scale = 120, cz = -1,
    f = function(x, y, z)
      local a, b, c, d, e, f = 0.95, 0.7, 0.6, 3.5, 0.25, 0.1
      return (z - b) * x - d * y, d * x + (z - b) * y,
             c + a * z - z ^ 3 / 3 - (x * x + y * y) * (1 + e * z) + f * z * x ^ 3
    end },
  { name = "Thomas", dt = 0.08, scale = 40, cz = 0,
    f = function(x, y, z) local b = 0.208186 return math.sin(y) - b * x, math.sin(z) - b * y, math.sin(x) - b * z end },
}
local which = 1
local pts, npts = {}, 6000
local x, y, z = 0.1, 0, 0
local ax, ay = 0.3, 0
local sweep = true
local hue0 = 200
local zoom = 1
local last_mx, last_my = 0, 0

local function reset()
  x, y, z = 0.1 + random() * 0.01, 0, 0
  pts = {}
  -- Skip the transient.
  local s = systems[which]
  for _ = 1, 2000 do
    local dx, dy, dz = s.f(x, y, z)
    x, y, z = x + dx * s.dt, y + dy * s.dt, z + dz * s.dt
  end
end
reset()

cls(rgb(2, 4, 12))
while true do
  if button_pressed("a") then which = which % #systems + 1 reset() cls(rgb(2, 4, 12)) end
  if button_pressed("b") then sweep = not sweep end
  if button("left") then ay = ay - 0.03 end
  if button("right") then ay = ay + 0.03 end
  if button("up") then ax = ax - 0.03 end
  if button("down") then ax = ax + 0.03 end
  local mx, my = mouse()
  if mouse_pressed() then last_mx, last_my = mx, my end
  if mouse_button() then
    ay = ay + (mx - last_mx) * 0.01
    ax = ax - (my - last_my) * 0.01
    last_mx, last_my = mx, my
  else
    ay = ay + 0.004
  end
  zoom = clamp(zoom * 1.1 ^ mouse_wheel(), 0.3, 4)
  if sweep then hue0 = (hue0 + 0.3) % 360 end

  local s = systems[which]
  -- Integrate a batch of new points, keep a sliding window.
  for _ = 1, 60 do
    local dx, dy, dz = s.f(x, y, z)
    x, y, z = x + dx * s.dt, y + dy * s.dt, z + dz * s.dt
    pts[#pts + 1] = { x, y, z }
  end
  while #pts > npts do table.remove(pts, 1) end

  -- Fade the screen (pixels persist, so a translucent fill leaves trails).
  blend("alpha")
  fill_rect(0, 0, w, h, rgb(2, 4, 12, 40))
  blend("add")
  local cy_, sy_ = math.cos(ay), math.sin(ay)
  local cx_, sx_ = math.cos(ax), math.sin(ax)
  local total = #pts
  for i, p in ipairs(pts) do
    local px, py, pz = p[1], p[2], p[3] + s.cz
    px, pz = px * cy_ + pz * sy_, -px * sy_ + pz * cy_
    py, pz = py * cx_ - pz * sx_, py * sx_ + pz * cx_
    local depth = 1 / (1 + pz / (s.scale * 6))
    local sx, sy = w / 2 + px * s.scale * zoom * depth, h / 2 - py * s.scale * zoom * depth
    local age = i / total
    local col = hsv((hue0 + age * 80 + pz) % 360, 0.85, 0.15 + 0.6 * age)
    if age > 0.97 then fill_rect(sx - 1, sy - 1, 2, 2, col) else pixel(sx, sy, col) end
  end
  blend("alpha")
  fill_rect(0, 0, 200, 14, rgb(2, 4, 12))
  draw_text(4, -1, s.name .. "  Z next  X hue  drag/wheel", rgb(150, 230, 255))
  flip()
end
