-- retro: a low resolution cart with a scrolling camera.
-- argon: resolution = "retro"   (320x240: chunky pixels on purpose)
alt_screen()
local w, h = screen_size()
local tiles = {}
for i = 1, 200 do tiles[i] = { x = i * 24, y = h - 40 - (i % 7) * 10 } end
local px, py, vy, on_ground = 40, h - 60, 0, false

while true do
  cls(rgb(70, 130, 180))
  if button("left") then px = px - 2 end
  if button("right") then px = px + 2 end
  if button_pressed("a") and on_ground then vy = -6 end
  vy = vy + 0.3
  py = py + vy
  on_ground = false
  for _, t in ipairs(tiles) do
    if px + 8 > t.x and px < t.x + 20 and py + 12 >= t.y and py + 12 <= t.y + 10 and vy >= 0 then
      py = t.y - 12 vy = 0 on_ground = true
    end
  end
  if py > h then py, vy = h - 60, 0 end

  camera(w / 2 - px, 0)
  for _, t in ipairs(tiles) do
    fill_rect(t.x, t.y, 20, 10, rgb(60, 160, 60))
    fill_rect(t.x, t.y, 20, 3, rgb(120, 220, 90))
  end
  fill_rect(px, py, 8, 12, rgb(240, 80, 60))
  fill_rect(px + 2, py + 3, 4, 2, rgb(255, 255, 255))
  camera()
  draw_text(4, 4, "LEFT/RIGHT move  Z jump  (320x240)", rgb(255, 255, 255))
  flip()
end
