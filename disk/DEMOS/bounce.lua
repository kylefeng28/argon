-- bounce: a ball, the pad buttons, and the main loop.
-- Arrow keys steer, A (Z key) speeds up, Esc quits.
alt_screen()
local w, h = screen_size()
local x, y = w / 2, h / 2
local dx, dy = 3, 2
local r = h // 25
local trail = {}

while true do
  cls(rgb(12, 12, 32))

  if button("left") then dx = dx - 0.2 end
  if button("right") then dx = dx + 0.2 end
  if button("up") then dy = dy - 0.2 end
  if button("down") then dy = dy + 0.2 end
  local speed = button("a") and 2 or 1

  x = x + dx * speed
  y = y + dy * speed
  if x < r or x > w - r then dx = -dx x = clamp(x, r, w - r) end
  if y < r or y > h - r then dy = -dy y = clamp(y, r, h - r) end

  table.insert(trail, { x, y })
  if #trail > 30 then table.remove(trail, 1) end
  for i, p in ipairs(trail) do
    local t = i / #trail
    fill_circle(p[1], p[2], r * t, hsv(time() * 90 + i * 6, 0.8, t))
  end
  fill_circle(x, y, r, rgb(255, 230, 120))

  draw_text(8, 8, "ARROWS steer   Z speeds up   ESC quits", rgb(160, 160, 200))
  draw_text(8, h - 16 * (h // 400) - 8, string.format("%.0f fps", 1 / math.max(delta_time(), 0.0001)), rgb(120, 120, 160))
  flip()
end
