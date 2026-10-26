-- julia: an animated Julia set, software rendered at 160x100 and scaled.
-- The complex parameter c orbits slowly; arrows nudge it, Z pauses.
alt_screen()
set_resolution("320x200")
local w, h = screen_size()
local step = 2                     -- render every other pixel as 2x2 blocks
local max_iter = 40
local t = 2.6
local paused = false
local ox, oy = 0, 0

while true do
  if button_pressed("a") then paused = not paused end
  if button("left") then ox = ox - 0.01 end
  if button("right") then ox = ox + 0.01 end
  if button("up") then oy = oy - 0.01 end
  if button("down") then oy = oy + 0.01 end
  if not paused then t = t + delta_time() * 0.25 end

  -- The classic tour: c moves around a circle of radius 0.7885, which
  -- stays near the edge of the Mandelbrot set where Julia sets are rich.
  local cr = 0.7885 * math.cos(t) + ox
  local ci = 0.7885 * math.sin(t) + oy

  for py = 0, h - 1, step do
    local zi0 = (py - h / 2) / (h / 2) * 1.2
    for px = 0, w - 1, step do
      local zr, zi = (px - w / 2) / (h / 2) * 1.2, zi0
      local i = 0
      while i < max_iter and zr * zr + zi * zi < 4 do
        zr, zi = zr * zr - zi * zi + cr, 2 * zr * zi + ci
        i = i + 1
      end
      local col
      if i >= max_iter then
        col = rgb(4, 6, 20)
      else
        local u = i / max_iter
        col = hsv(195 + u * 40, 1 - u * 0.85, 0.04 + 0.96 * u ^ 1.2)
      end
      fill_rect(px, py, step, step, col)
    end
  end
  draw_text(2, 2, string.format("c = %.3f %+.3fi", cr, ci), rgb(230, 250, 255))
  draw_text(2, h - 16, paused and "PAUSED  Z resume" or "Z pause  ARROWS nudge c", rgb(120, 180, 200))
  flip()
end
