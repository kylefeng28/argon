-- julia: an animated Julia set, software rendered at full 320x200.
-- Each frame renders as many rows as fit in 1/60 s; when the image is
-- complete, c moves on. Arrows nudge c, Z pauses, X toggles half-res.
alt_screen()
set_resolution("320x200")
local w, h = screen_size()
local max_iter = 48
local t = 2.6
local paused = false
local ox, oy = 0, 0
local step = 1                    -- 1 = every pixel, 2 = 2x2 blocks
local row = 0
local rows_per_frame = 20
local cr, ci = 0, 0
local images, last_fps_t = 0, time()
local fps_shown = 0

local function new_c()
  -- The classic tour: c moves around a circle of radius 0.7885, which
  -- stays near the edge of the Mandelbrot set where Julia sets are rich.
  cr = 0.7885 * math.cos(t) + ox
  ci = 0.7885 * math.sin(t) + oy
end
new_c()

local function render_rows(n)
  for _ = 1, n do
    if row >= h then return end
    local zi0 = (row - h / 2) / (h / 2) * 1.2
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
      if step == 1 then pixel(px, row, col) else fill_rect(px, row, step, step, col) end
    end
    row = row + step
  end
end

while true do
  if button_pressed("a") then paused = not paused end
  if button_pressed("b") then step = 3 - step row = 0 end
  if button("left") then ox = ox - 0.005 end
  if button("right") then ox = ox + 0.005 end
  if button("up") then oy = oy - 0.005 end
  if button("down") then oy = oy + 0.005 end

  render_rows(rows_per_frame)
  if row >= h then
    -- Image complete: advance the animation and start over.
    images = images + 1
    if time() - last_fps_t >= 1 then
      fps_shown, images, last_fps_t = images, 0, time()
    end
    if not paused then t = t + 0.02 end
    new_c()
    row = 0
  end
  -- Adapt work per frame to hold 60 fps.
  if delta_time() > 0.019 then rows_per_frame = math.max(1, rows_per_frame - 2)
  elseif delta_time() < 0.014 then rows_per_frame = math.min(h, rows_per_frame + 2) end

  fill_rect(0, 0, w, 12, rgb(4, 6, 20))
  draw_text(2, -2, string.format("c = %.3f %+.3fi   %d images/s", cr, ci, fps_shown), rgb(230, 250, 255))
  fill_rect(0, h - 12, w, 12, rgb(4, 6, 20))
  draw_text(2, h - 14, (paused and "PAUSED " or "") .. "Z pause  X " .. (step == 1 and "half-res" or "full-res") .. "  ARROWS nudge c", rgb(120, 180, 200))
  flip()
end
