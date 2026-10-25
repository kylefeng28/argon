-- plot: graph a function, like the TI-84 graph screen.
alt_screen()
local w, h = screen_size()
local xmin, xmax, ymin, ymax = -10, 10, -5, 5

local function sx(x) return (x - xmin) / (xmax - xmin) * w end
local function sy(y) return h - (y - ymin) / (ymax - ymin) * h end

local funcs = {
  { "y = sin(x)", function(x) return math.sin(x) end, rgb(255, 120, 120) },
  { "y = x^2 / 10", function(x) return x * x / 10 end, rgb(120, 255, 120) },
  { "y = tan(x)", function(x) return math.tan(x) end, rgb(120, 160, 255) },
  { "y = 3 sin(x) cos(2x)", function(x) return 3 * math.sin(x) * math.cos(2 * x) end, rgb(255, 220, 120) },
}
local which = 1

while true do
  cls(rgb(230, 235, 225))
  local grey = rgb(170, 175, 165)
  for gx = math.ceil(xmin), math.floor(xmax) do line(sx(gx), 0, sx(gx), h, grey) end
  for gy = math.ceil(ymin), math.floor(ymax) do line(0, sy(gy), w, sy(gy), grey) end
  line(sx(0), 0, sx(0), h, rgb(0, 0, 0))
  line(0, sy(0), w, sy(0), rgb(0, 0, 0))

  local f = funcs[which]
  local prev
  for px = 0, w do
    local x = xmin + px / w * (xmax - xmin)
    local y = f[2](x)
    if y == y and math.abs(y) < 1e6 then
      local py = sy(y)
      if prev and math.abs(py - prev) < h then line(px - 1, prev, px, py, f[3]) end
      prev = py
    else
      prev = nil
    end
  end
  draw_text(8, 8, f[1], rgb(0, 0, 0), 2)
  draw_text(8, h - 16 * (h // 400) - 8, "LEFT/RIGHT: change function   ESC: quit", rgb(80, 80, 80))
  if button_pressed("right") then which = which % #funcs + 1 end
  if button_pressed("left") then which = (which - 2) % #funcs + 1 end
  flip()
end
