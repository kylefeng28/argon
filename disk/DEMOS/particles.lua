-- particles: 20,000 pixels moving at 60 fps. Hold A (Z) to attract them.
alt_screen()
local w, h = screen_size()
local n = 20000 * (h // 400)
local px, py, vx, vy = {}, {}, {}, {}
for i = 1, n do
  px[i], py[i] = random() * w, random() * h
  vx[i], vy[i] = (random() - 0.5) * 2, (random() - 0.5) * 2
end

while true do
  cls(rgb(0, 0, 0))
  local cx, cy = w / 2 + math.cos(time()) * 150, h / 2 + math.sin(time() * 1.3) * 100
  local attract = button("a")
  for i = 1, n do
    local x, y = px[i], py[i]
    if attract then
      local ddx, ddy = cx - x, cy - y
      local d = math.sqrt(ddx * ddx + ddy * ddy) + 1
      vx[i] = vx[i] + ddx / d * 0.3
      vy[i] = vy[i] + ddy / d * 0.3
    end
    vx[i] = vx[i] * 0.995
    vy[i] = vy[i] * 0.995
    x, y = x + vx[i], y + vy[i]
    if x < 0 then x = x + w elseif x >= w then x = x - w end
    if y < 0 then y = y + h elseif y >= h then y = y - h end
    px[i], py[i] = x, y
    local speed = math.abs(vx[i]) + math.abs(vy[i])
    pixel(x, y, hsv(200 - speed * 40, 0.8, 1))
  end
  draw_text(8, 8, string.format("%d particles  %.0f fps  hold Z to attract", n, 1 / math.max(delta_time(), 0.0001)), rgb(200, 200, 200))
  flip()
end
