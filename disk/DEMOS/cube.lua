-- cube: a rotating 3D cube rendered in software with 2D primitives.
-- Z cycles the material: wireframe, flat shaded, textured. X toggles the
-- wire overlay. Arrows spin the cube by hand.
alt_screen()
local w, h = screen_size()

-- Geometry: 8 vertices, 6 quad faces (counter-clockwise seen from outside).
local verts = {
  { -1, -1, -1 }, { 1, -1, -1 }, { 1, 1, -1 }, { -1, 1, -1 },
  { -1, -1, 1 }, { 1, -1, 1 }, { 1, 1, 1 }, { -1, 1, 1 },
}
local faces = {
  { 1, 4, 3, 2, color = rgb(230, 80, 80) },   -- back  (z = -1)
  { 5, 6, 7, 8, color = rgb(80, 200, 100) },  -- front (z = +1)
  { 1, 2, 6, 5, color = rgb(80, 120, 240) },  -- bottom
  { 4, 8, 7, 3, color = rgb(240, 220, 80) },  -- top
  { 1, 5, 8, 4, color = rgb(230, 130, 60) },  -- left
  { 2, 3, 7, 6, color = rgb(190, 90, 220) },  -- right
}

-- A procedural texture: a tile with a border and a diagonal.
local tex = new_image(64, 64)
set_target(tex)
cls(rgb(210, 190, 150))
for i = 0, 7 do
  for j = 0, 7 do
    if (i + j) % 2 == 0 then fill_rect(i * 8, j * 8, 8, 8, rgb(150, 110, 70)) end
  end
end
rect(0, 0, 64, 64, rgb(60, 40, 20))
line_width(3)
line(2, 2, 61, 61, rgb(60, 40, 20))
line_width(1)
set_target()

local mode, wire_overlay = 1, false
local modes = { "wireframe", "flat shaded", "textured" }
local ax, ay = 0.5, 0.3
local auto = true
local light = { 0.4, -0.7, -0.6 } -- direction toward the light
local ll = math.sqrt(light[1] ^ 2 + light[2] ^ 2 + light[3] ^ 2)
light = { light[1] / ll, light[2] / ll, light[3] / ll }

local function rotate(v, ax, ay)
  local x, y, z = v[1], v[2], v[3]
  local cy, sy = math.cos(ay), math.sin(ay)
  x, z = x * cy + z * sy, -x * sy + z * cy
  local cx, sx = math.cos(ax), math.sin(ax)
  y, z = y * cx - z * sx, y * sx + z * cx
  return x, y, z
end

local function project(x, y, z)
  local d = 4.5
  local s = (h / 2) / (z + d) * 1.3
  return w / 2 + x * s, h / 2 + y * s
end

while true do
  if button_pressed("a") then mode = mode % #modes + 1 end
  if button_pressed("b") then wire_overlay = not wire_overlay end
  local dt = delta_time()
  local manual = false
  if button("left") then ay = ay - 2 * dt manual = true end
  if button("right") then ay = ay + 2 * dt manual = true end
  if button("up") then ax = ax - 2 * dt manual = true end
  if button("down") then ax = ax + 2 * dt manual = true end
  if button_pressed("start") then auto = not auto end
  if auto and not manual then
    ay = ay + 0.9 * dt
    ax = ax + 0.4 * dt
  end

  cls(rgb(14, 16, 28))
  -- Floor grid for depth.
  for i = -6, 6 do
    local x1, y1 = project(i * 0.6, 1.6, -3)
    local x2, y2 = project(i * 0.6, 1.6, 3)
    line(x1, y1, x2, y2, rgb(30, 34, 60))
  end
  for j = -3, 3 do
    local x1, y1 = project(-3.6, 1.6, j)
    local x2, y2 = project(3.6, 1.6, j)
    line(x1, y1, x2, y2, rgb(30, 34, 60))
  end

  -- Transform all vertices once.
  local rv, sv = {}, {}
  for i, v in ipairs(verts) do
    local x, y, z = rotate(v, ax, ay)
    rv[i] = { x, y, z }
    local px, py = project(x, y, z)
    sv[i] = { px, py }
  end

  -- Visible faces with depth and lighting, painter's algorithm.
  local drawlist = {}
  for _, f in ipairs(faces) do
    local a, b, c = rv[f[1]], rv[f[2]], rv[f[3]]
    local ux, uy, uz = b[1] - a[1], b[2] - a[2], b[3] - a[3]
    local vx, vy, vz = c[1] - a[1], c[2] - a[2], c[3] - a[3]
    local nx, ny, nz = uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx
    local nl = math.sqrt(nx * nx + ny * ny + nz * nz)
    nx, ny, nz = nx / nl, ny / nl, nz / nl
    -- Camera looks down +z from the viewer at negative z: faces whose normal
    -- points toward the viewer (nz < 0) are visible.
    local facing = nz < 0
    if facing or mode == 1 then
      local depth = (a[3] + b[3] + c[3] + rv[f[4]][3]) / 4
      local shade = clamp(0.25 + 0.75 * math.max(0, nx * light[1] + ny * light[2] + nz * light[3]), 0, 1)
      drawlist[#drawlist + 1] = { f = f, depth = depth, shade = shade, facing = facing }
    end
  end
  table.sort(drawlist, function(p, q) return p.depth > q.depth end)

  for _, d in ipairs(drawlist) do
    local f = d.f
    local p1, p2, p3, p4 = sv[f[1]], sv[f[2]], sv[f[3]], sv[f[4]]
    if mode == 2 and d.facing then
      local c = f.color
      local col = rgb(((c // 2 ^ 24) % 256) * d.shade, ((c // 2 ^ 16) % 256) * d.shade, ((c // 2 ^ 8) % 256) * d.shade)
      fill_polygon({ p1[1], p1[2], p2[1], p2[2], p3[1], p3[2], p4[1], p4[2] }, col)
    elseif mode == 3 and d.facing then
      draw_image_quad(tex, p1[1], p1[2], p2[1], p2[2], p3[1], p3[2], p4[1], p4[2])
      -- Darken with a translucent black polygon for lighting.
      local dark = math.floor((1 - d.shade) * 200)
      if dark > 0 then
        fill_polygon({ p1[1], p1[2], p2[1], p2[2], p3[1], p3[2], p4[1], p4[2] }, rgb(0, 0, 0, dark))
      end
    end
    if mode == 1 or wire_overlay then
      local col = d.facing and rgb(120, 255, 180) or rgb(50, 90, 70)
      polygon({ p1[1], p1[2], p2[1], p2[2], p3[1], p3[2], p4[1], p4[2] }, col)
    end
  end
  for i = 1, 8 do
    if mode == 1 then fill_rect(sv[i][1] - 1, sv[i][2] - 1, 3, 3, rgb(255, 255, 255)) end
  end

  draw_text(8, 8, "CUBE  " .. modes[mode] .. (wire_overlay and " + wire" or ""), rgb(255, 255, 255), 2)
  draw_text(8, h - 20, "Z material   X wire overlay   ARROWS spin   ENTER auto-rotate", rgb(140, 140, 180))
  flip()
end
