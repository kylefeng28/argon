-- mandelbrot: software rendered, progressively, at 320x240.
-- Arrows pan, Z zooms in, X zooms out, C cycles the palette, R resets.
alt_screen()
set_resolution("retro")
local w, h = screen_size()

local cx, cy, zoom = -0.6, 0.0, 1.0
local max_iter = 64
local palette_mode = 1
local row = 0                      -- next row to render
local rows_per_frame = 6

local function color_for(i, zr, zi)
  if i >= max_iter then return rgb(0, 0, 0) end
  -- Smooth iteration count for continuous shading.
  local log_zn = math.log(zr * zr + zi * zi) / 2
  local nu = math.log(log_zn / math.log(2)) / math.log(2)
  local t = (i + 1 - nu) / max_iter
  if palette_mode == 1 then
    -- The Grid: far away is near-black blue, closer turns cyan, then
    -- white-hot, with an orange rim right at the edge of the set.
    local u = clamp(t, 0, 1) ^ 0.7
    if u < 0.55 then
      local k = (u / 0.55) ^ 1.5
      return rgb(2 + 20 * k, 6 + 182 * k, 18 + 226 * k)
    elseif u < 0.85 then
      local k = (u - 0.55) / 0.3
      return rgb(22 + 233 * k, 188 + 67 * k, 244 + 11 * k)
    else
      local k = (u - 0.85) / 0.15
      return rgb(255, 255 - 125 * k, 255 - 230 * k)
    end
  elseif palette_mode == 2 then
    return hsv(t * 360 * 3 % 360, 0.8, 1)
  else
    local g = math.floor(255 * math.sqrt(t))
    return rgb(g, g, g)
  end
end

local function restart()
  row = 0
  cls(rgb(0, 0, 0))
end

local function render_rows(n)
  local scale = 3.0 / zoom
  for _ = 1, n do
    if row >= h then return end
    local ci = cy + (row - h / 2) * (scale / w)
    for px = 0, w - 1 do
      local cr = cx + (px - w / 2) * (scale / w)
      local zr, zi, i = 0, 0, 0
      while i < max_iter and zr * zr + zi * zi < 16 do
        zr, zi = zr * zr - zi * zi + cr, 2 * zr * zi + ci
        i = i + 1
      end
      pixel(px, row, color_for(i, zr, zi))
    end
    row = row + 1
  end
end

restart()
while true do
  local moved = false
  local pan = 0.08 / zoom
  if button("left") then cx = cx - pan moved = true end
  if button("right") then cx = cx + pan moved = true end
  if button("up") then cy = cy - pan moved = true end
  if button("down") then cy = cy + pan moved = true end
  if button("a") then zoom = zoom * 1.08 moved = true end
  if button("b") then zoom = zoom / 1.08 moved = true end
  if button_pressed("x") then palette_mode = palette_mode % 3 + 1 moved = true end
  if key_pressed("r") then cx, cy, zoom = -0.6, 0.0, 1.0 moved = true end
  if moved then
    max_iter = math.floor(clamp(64 + math.log(zoom, 2) * 24, 64, 1024))
    restart()
  end

  -- Keep rendering rows while there is time; the framebuffer persists.
  render_rows(rows_per_frame)
  -- Adapt work per frame to keep ~60 fps.
  if delta_time() > 0.02 then rows_per_frame = math.max(1, rows_per_frame - 1)
  elseif delta_time() < 0.012 then rows_per_frame = math.min(60, rows_per_frame + 1) end

  -- Progress bar and HUD on top (small, so most of the image stays visible).
  fill_rect(0, h - 18, w, 18, rgb(0, 0, 0))
  fill_rect(0, h - 18, w * row / h, 2, rgb(0, 220, 255))
  draw_text(2, h - 16, string.format("%.5f %+.5fi  zoom %.0f  iter %d", cx, cy, zoom, max_iter), rgb(120, 200, 230))
  flip()
end
