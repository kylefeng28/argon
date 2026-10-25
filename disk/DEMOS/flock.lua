-- flock: boids (separation, alignment, cohesion) with additive light trails.
-- Hold Z to attract the flock to a wandering point, X to scatter them.
-- Up/Down change the flock size.
alt_screen()
local w, h = screen_size()
local n = 220 * (h // 400)
local boids = {}
local function spawn(i)
  local a = random() * math.pi * 2
  boids[i] = { x = random() * w, y = random() * h, vx = math.cos(a) * 2, vy = math.sin(a) * 2, hue = random() * 360 }
end
for i = 1, n do spawn(i) end

local max_speed, min_speed = 3.2 * (h / 400), 1.2 * (h / 400)
local view, sep_dist = 42 * (h / 400), 14 * (h / 400)
local grid = {}
local gs = view

-- Trails: draw into an image, fade it a little each frame, then draw it.
local trail = new_image(w, h)
set_target(trail) cls(rgb(0, 0, 0)) set_target()

local target = { x = w / 2, y = h / 2 }
cls(rgb(2, 4, 12))
while true do
  if button_pressed("up") then for i = n + 1, math.min(n + 40, 600) do spawn(i) end n = math.min(n + 40, 600) end
  if button_pressed("down") then n = math.max(n - 40, 40) end
  local attract, scatter = button("a"), button("b")
  target.x = w / 2 + math.cos(time() * 0.6) * w * 0.35
  target.y = h / 2 + math.sin(time() * 0.9) * h * 0.35

  -- Spatial hash so neighbor search stays cheap.
  for k in pairs(grid) do grid[k] = nil end
  for i = 1, n do
    local b = boids[i]
    local k = (b.x // gs) + (b.y // gs) * 1000
    local cell = grid[k]
    if not cell then cell = {} grid[k] = cell end
    cell[#cell + 1] = b
  end

  for i = 1, n do
    local b = boids[i]
    local ax, ay, cx, cy, sx, sy, count = 0, 0, 0, 0, 0, 0, 0
    local gx, gy = b.x // gs, b.y // gs
    for dy = -1, 1 do
      for dx = -1, 1 do
        local cell = grid[(gx + dx) + (gy + dy) * 1000]
        if cell then
          for _, o in ipairs(cell) do
            if o ~= b then
              local ox, oy = o.x - b.x, o.y - b.y
              local d2 = ox * ox + oy * oy
              if d2 < view * view then
                count = count + 1
                ax, ay = ax + o.vx, ay + o.vy
                cx, cy = cx + o.x, cy + o.y
                if d2 < sep_dist * sep_dist and d2 > 0 then
                  local inv = 1 / math.sqrt(d2)
                  sx, sy = sx - ox * inv, sy - oy * inv
                end
              end
            end
          end
        end
      end
    end
    local fx, fy = 0, 0
    if count > 0 then
      fx = fx + (ax / count - b.vx) * 0.05 + (cx / count - b.x) * 0.004 + sx * 0.6
      fy = fy + (ay / count - b.vy) * 0.05 + (cy / count - b.y) * 0.004 + sy * 0.6
    end
    if attract then fx, fy = fx + (target.x - b.x) * 0.002, fy + (target.y - b.y) * 0.002 end
    if scatter then
      local dx, dy = b.x - target.x, b.y - target.y
      local d = math.sqrt(dx * dx + dy * dy) + 1
      fx, fy = fx + dx / d * 0.5, fy + dy / d * 0.5
    end
    -- Soft walls.
    local m = h / 10
    if b.x < m then fx = fx + 0.08 elseif b.x > w - m then fx = fx - 0.08 end
    if b.y < m then fy = fy + 0.08 elseif b.y > h - m then fy = fy - 0.08 end
    b.vx, b.vy = b.vx + fx, b.vy + fy
    local sp = math.sqrt(b.vx * b.vx + b.vy * b.vy)
    if sp > max_speed then b.vx, b.vy = b.vx / sp * max_speed, b.vy / sp * max_speed
    elseif sp < min_speed and sp > 0 then b.vx, b.vy = b.vx / sp * min_speed, b.vy / sp * min_speed end
    b.x, b.y = b.x + b.vx, b.y + b.vy
    -- Hue drifts toward the heading so flocks share a color.
    local heading = math.deg(math.atan2(b.vy, b.vx)) % 360
    b.hue = b.hue + ((heading - b.hue + 540) % 360 - 180) * 0.05
  end

  -- Fade the trail image, then add this frame's segments to it.
  set_target(trail)
  blend("alpha")
  fill_rect(0, 0, w, h, rgb(2, 4, 12, 28))
  blend("add")
  for i = 1, n do
    local b = boids[i]
    local col = hsv(b.hue, 0.9, 0.35)
    line(b.x - b.vx * 3, b.y - b.vy * 3, b.x, b.y, col)
  end
  blend("alpha")
  set_target()

  cls(rgb(2, 4, 12))
  draw_image(trail, 0, 0)
  for i = 1, n do
    local b = boids[i]
    local sp = math.sqrt(b.vx * b.vx + b.vy * b.vy)
    local ux, uy = b.vx / sp, b.vy / sp
    local col = hsv(b.hue, 0.5, 1)
    fill_triangle(b.x + ux * 5, b.y + uy * 5, b.x - ux * 3 - uy * 2.5, b.y - uy * 3 + ux * 2.5, b.x - ux * 3 + uy * 2.5, b.y - uy * 3 - ux * 2.5, col)
  end
  if attract or scatter then
    circle(target.x, target.y, 8 + math.sin(time() * 8) * 3, attract and rgb(120, 240, 255) or rgb(255, 120, 40))
  end
  draw_text(4, 2, string.format("FLOCK  %d boids  %.0f fps", n, 1 / math.max(delta_time(), 1e-4)), rgb(150, 230, 255))
  draw_text(4, h - 16 * (h // 400) - 2, "hold Z attract  X scatter  UP/DOWN size", rgb(80, 130, 160))
  flip()
end
