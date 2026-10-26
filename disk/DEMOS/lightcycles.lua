-- lightcycles: you (cyan) against the program (orange) on the Grid.
-- Arrows turn. Your ribbon is a wall; so is theirs. Z restarts.
alt_screen()
local w, h = screen_size()
local cell = 4                       -- grid step in pixels
local cols, rows = w // cell, (h - 40) // cell
local top = 40                       -- HUD height

local function setup()
  local walls = {}                   -- walls[y*cols+x] = owner
  local p1 = { x = cols // 4, y = rows // 2, dx = 1, dy = 0, color = rgb(90, 230, 255), glow = rgb(200, 250, 255), alive = true, name = "YOU" }
  local p2 = { x = cols * 3 // 4, y = rows // 2, dx = -1, dy = 0, color = rgb(255, 130, 30), glow = rgb(255, 220, 160), alive = true, name = "CLU" }
  return walls, p1, p2
end
local walls, p1, p2 = setup()
local score = { 0, 0 }
local state, state_t = "ready", 0
local speed, acc = 22, 0            -- cells per second

local function blocked(x, y)
  return x < 0 or y < 0 or x >= cols or y >= rows or walls[y * cols + x] ~= nil
end

local function px(x) return x * cell end
local function py(y) return top + y * cell end

local function draw_arena()
  cls(rgb(2, 6, 14))
  -- Floor: faint grid lines like the arena.
  for x = 0, w, 40 do line(x, top, x, h, rgb(10, 30, 46)) end
  for y = top, h, 40 do line(0, y, w, y, rgb(10, 30, 46)) end
  line_width(2)
  rect(1, top + 1, w - 2, h - top - 2, rgb(0, 150, 190))
  line_width(1)
end

local function draw_wall(x, y, p)
  fill_rect(px(x), py(y), cell, cell, p.color)
end

local function draw_head(p)
  -- Same size as a wall cell so the ribbon overwrites it cleanly.
  fill_rect(px(p.x), py(p.y), cell, cell, p.glow)
end

-- Simple AI: keep going unless blocked soon; prefer the turn with more room.
local function room(x, y, dx, dy)
  local n = 0
  while n < 40 and not blocked(x + dx * (n + 1), y + dy * (n + 1)) do n = n + 1 end
  return n
end

local function think(p)
  local ahead = room(p.x, p.y, p.dx, p.dy)
  local left_dx, left_dy = p.dy, -p.dx
  local right_dx, right_dy = -p.dy, p.dx
  local l, r = room(p.x, p.y, left_dx, left_dy), room(p.x, p.y, right_dx, right_dy)
  if ahead < 3 or (ahead < 12 and random() < 0.15) then
    if l > r then p.dx, p.dy = left_dx, left_dy
    elseif r > 0 then p.dx, p.dy = right_dx, right_dy end
  elseif random() < 0.01 and math.max(l, r) > 8 then
    if l > r then p.dx, p.dy = left_dx, left_dy else p.dx, p.dy = right_dx, right_dy end
  end
end

local function step(p, owner)
  walls[p.y * cols + p.x] = owner
  draw_wall(p.x, p.y, p)
  local nx, ny = p.x + p.dx, p.y + p.dy
  if blocked(nx, ny) then p.alive = false return end
  p.x, p.y = nx, ny
end

draw_arena()
while true do
  local dt = delta_time()
  if state == "ready" then
    state_t = state_t + dt
    if state_t > 1.2 then state = "run" draw_arena() end
  elseif state == "run" then
    -- Turning: never reverse into yourself.
    if button_pressed("up") and p1.dy == 0 then p1.dx, p1.dy = 0, -1 end
    if button_pressed("down") and p1.dy == 0 then p1.dx, p1.dy = 0, 1 end
    if button_pressed("left") and p1.dx == 0 then p1.dx, p1.dy = -1, 0 end
    if button_pressed("right") and p1.dx == 0 then p1.dx, p1.dy = 1, 0 end
    acc = acc + dt * speed
    while acc >= 1 do
      acc = acc - 1
      think(p2)
      step(p1, 1)
      step(p2, 2)
      if p1.x == p2.x and p1.y == p2.y then p1.alive, p2.alive = false, false end
      if not p1.alive or not p2.alive then
        state, state_t = "over", 0
        if p1.alive then score[1] = score[1] + 1 elseif p2.alive then score[2] = score[2] + 1 end
        beep(p1.alive and 880 or 160, 0.4, { wave = p1.alive and "square" or "saw", slide = p1.alive and 400 or -200, volume = 0.5 })
        break
      end
    end
    if frame() % 4 == 0 then tone(1, { freq = 110 + speed * 2, wave = "saw", volume = 0.05 }) end
  elseif state == "over" then
    tone(1)
    state_t = state_t + dt
    if button_pressed("a") or state_t > 4 then
      walls, p1, p2 = setup()
      speed = speed + 2
      state, state_t, acc = "ready", 0, 0
      draw_arena()
    end
  end

  -- HUD (redrawn every frame on top of the persistent arena).
  fill_rect(0, 0, w, top, rgb(2, 6, 14))
  line(0, top - 1, w, top - 1, rgb(0, 150, 190))
  draw_text(12, 10, string.format("YOU %d", score[1]), p1.color, 2)
  draw_text(w - 12 - 6 * 16, 10, string.format("%d CLU", score[2]), p2.color, 2)
  draw_text(w / 2 - 88, 14, "L I G H T   C Y C L E S", rgb(180, 240, 255))
  draw_head(p1)
  draw_head(p2)
  if state == "ready" then
    draw_text(w / 2 - 48, h / 2 - 8, "GET READY", rgb(255, 255, 255), 2)
  elseif state == "over" then
    local msg = p1.alive and "YOU WIN" or (p2.alive and "DEREZZED" or "DRAW")
    fill_rect(w / 2 - 120, h / 2 - 30, 240, 60, rgb(2, 6, 14))
    rect(w / 2 - 120, h / 2 - 30, 240, 60, rgb(0, 200, 240))
    draw_text(w / 2 - #msg * 8, h / 2 - 20, msg, p1.alive and p1.color or p2.color, 2)
    draw_text(w / 2 - 60, h / 2 + 8, "Z to play again", rgb(150, 200, 220))
  end
  flip()
end
