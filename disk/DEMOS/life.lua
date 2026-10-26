-- life: Conway's Game of Life on a 160x100 grid, 4 px cells.
-- Cells glow brighter the longer they live; the dead leave fading trails.
-- Z pauses, X steps, C clears, R reseeds, 1-3 drop a glider gun / pulsar / R-pentomino.
-- Mouse: left button paints live cells, right button erases, wheel changes speed.
alt_screen()
local w, h = screen_size()
local cs = 4
local cols, rows = w // cs, h // cs
local N = cols * rows

local alive, age, heat = {}, {}, {}
local function clear()
  for i = 0, N - 1 do alive[i], age[i], heat[i] = 0, 0, 0 end
end
local function seed(density)
  for i = 0, N - 1 do alive[i] = random() < density and 1 or 0 age[i] = 0 end
end

local function stamp(pattern, x0, y0)
  for y, line in ipairs(pattern) do
    for x = 1, #line do
      if line:sub(x, x) == "O" then
        local cx, cy = (x0 + x - 1) % cols, (y0 + y - 1) % rows
        alive[cy * cols + cx] = 1
      end
    end
  end
end

local gun = {
  "........................O...........",
  "......................O.O...........",
  "............OO......OO............OO",
  "...........O...O....OO............OO",
  "OO........O.....O...OO..............",
  "OO........O...O.OO....O.O...........",
  "..........O.....O.......O...........",
  "...........O...O....................",
  "............OO......................",
}
local pulsar = {
  "..OOO...OOO..", ".............", "O....O.O....O", "O....O.O....O", "O....O.O....O", "..OOO...OOO..",
  ".............", "..OOO...OOO..", "O....O.O....O", "O....O.O....O", "O....O.O....O", ".............", "..OOO...OOO..",
}
local rpent = { ".OO", "OO.", ".O." }

clear()
seed(0.18)
local paused, gen = false, 0
local next_alive = {}
local speed, acc = 20, 0 -- generations per second

local function step()
  for y = 0, rows - 1 do
    local ym, yp = ((y - 1) % rows) * cols, ((y + 1) % rows) * cols
    local yc = y * cols
    for x = 0, cols - 1 do
      local xm, xp = (x - 1) % cols, (x + 1) % cols
      local n = alive[ym + xm] + alive[ym + x] + alive[ym + xp]
              + alive[yc + xm] + alive[yc + xp]
              + alive[yp + xm] + alive[yp + x] + alive[yp + xp]
      local i = yc + x
      local a = alive[i]
      if n == 3 or (a == 1 and n == 2) then
        next_alive[i] = 1
        age[i] = a == 1 and age[i] + 1 or 1
      else
        next_alive[i] = 0
        if a == 1 then heat[i] = 1 end
        age[i] = 0
      end
    end
  end
  alive, next_alive = next_alive, alive
  gen = gen + 1
end

while true do
  if button_pressed("a") then paused = not paused end
  if button_pressed("b") then step() end
  if key_pressed("c") then clear() gen = 0 end
  if key_pressed("r") then clear() seed(0.18) gen = 0 end
  if key_pressed("1") then stamp(gun, 4, 4) end
  if key_pressed("2") then stamp(pulsar, cols // 2 - 6, rows // 2 - 6) end
  if key_pressed("3") then stamp(rpent, random(10, cols - 10), random(10, rows - 10)) end
  if button("up") then speed = math.min(speed + 1, 120) end
  if button("down") then speed = math.max(speed - 1, 1) end
  speed = clamp(speed + mouse_wheel() * 4, 1, 120)
  local mx, my, inside = mouse()
  if inside and (mouse_button("left") or mouse_button("right")) then
    local cx, cy = mx // cs, my // cs
    for dy = -1, 1 do
      for dx = -1, 1 do
        local i = ((cy + dy) % rows) * cols + (cx + dx) % cols
        if mouse_button("left") then alive[i] = 1 age[i] = 0 else alive[i] = 0 end
      end
    end
  end

  if not paused then
    acc = acc + delta_time() * speed
    while acc >= 1 do acc = acc - 1 step() end
  end

  cls(rgb(2, 6, 14))
  for i = 0, N - 1 do
    local x, y = (i % cols) * cs, (i // cols) * cs
    if alive[i] == 1 then
      -- Newborn cells are white-hot, settle through cyan to deep blue as they age.
      local a = math.min(age[i], 30) / 30
      local col = a < 0.15 and rgb(255, 255, 255) or hsv(185 + a * 35, 0.6 + a * 0.4, 1 - a * 0.55)
      fill_rect(x, y, cs - 1, cs - 1, col)
    elseif heat[i] > 0.02 then
      -- Afterglow in orange, fading out.
      local t = heat[i]
      fill_rect(x + 1, y + 1, cs - 2, cs - 2, rgb(255 * t, 110 * t, 20 * t))
      heat[i] = t * 0.85
    else
      heat[i] = 0
    end
  end
  fill_rect(0, 0, w, 14, rgb(2, 6, 14))
  draw_text(4, -1, string.format("LIFE  gen %d  %d gen/s%s", gen, speed, paused and "  PAUSED" or ""), rgb(150, 230, 255))
  draw_text(w - 420, -1, "Z pause  X step  C clear  R seed  1 gun  2 pulsar  3 R", rgb(80, 130, 160))
  if inside then
    -- A crosshair where the mouse is (Argon draws no cursor).
    local cx, cy = (mx // cs) * cs, (my // cs) * cs
    rect(cx - cs, cy - cs, cs * 3, cs * 3, rgb(255, 255, 255))
  end
  flip()
end
