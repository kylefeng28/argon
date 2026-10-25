-- starfield: the demo every machine gets first.
local w, h = screen_size()
local stars = {}
for i = 1, 400 do
  stars[i] = { x = random() * 2 - 1, y = random() * 2 - 1, z = random() }
end
local speed = 0.01

while true do
  cls(rgb(0, 0, 0))
  if button("up") then speed = math.min(speed + 0.001, 0.05) end
  if button("down") then speed = math.max(speed - 0.001, 0.001) end
  for _, s in ipairs(stars) do
    s.z = s.z - speed
    if s.z <= 0.01 then s.x, s.y, s.z = random() * 2 - 1, random() * 2 - 1, 1 end
    local sx = w / 2 + s.x / s.z * w / 2
    local sy = h / 2 + s.y / s.z * h / 2
    local b = math.floor(255 * (1 - s.z))
    if s.z < 0.3 then
      fill_rect(sx, sy, 2, 2, rgb(b, b, 255))
    else
      pixel(sx, sy, rgb(b, b, b))
    end
  end
  draw_text(8, 8, "UP/DOWN: speed", rgb(120, 120, 120))
  flip()
end
