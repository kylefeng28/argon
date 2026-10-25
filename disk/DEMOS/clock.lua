-- clock: an analog stopwatch since the program started.
local w, h = screen_size()
local cx, cy, r = w / 2, h / 2, 150

while true do
  cls(rgb(30, 30, 36))
  fill_circle(cx, cy, r + 6, rgb(200, 200, 210))
  fill_circle(cx, cy, r, rgb(250, 250, 245))
  for i = 0, 59 do
    local a = i / 60 * math.pi * 2
    local len = (i % 5 == 0) and 14 or 6
    line(cx + math.sin(a) * (r - len), cy - math.cos(a) * (r - len), cx + math.sin(a) * (r - 2), cy - math.cos(a) * (r - 2), rgb(40, 40, 40))
  end
  local t = time()
  local sec = t % 60
  local min = (t / 60) % 60
  local hr = (t / 3600) % 12
  line_width(6)
  local a = hr / 12 * math.pi * 2
  line(cx, cy, cx + math.sin(a) * r * 0.5, cy - math.cos(a) * r * 0.5, rgb(20, 20, 20))
  line_width(4)
  a = min / 60 * math.pi * 2
  line(cx, cy, cx + math.sin(a) * r * 0.75, cy - math.cos(a) * r * 0.75, rgb(20, 20, 20))
  line_width(2)
  a = sec / 60 * math.pi * 2
  line(cx, cy, cx + math.sin(a) * r * 0.9, cy - math.cos(a) * r * 0.9, rgb(220, 40, 40))
  line_width(1)
  fill_circle(cx, cy, 6, rgb(220, 40, 40))
  draw_text(8, 8, string.format("%02d:%02d:%05.2f", math.floor(hr), math.floor(min), sec), rgb(255, 255, 255), 2)
  flip()
end
