-- sprites: build a sprite sheet with set_target, then draw rotated sprites.
alt_screen()
local w, h = screen_size()

-- Draw four 32x32 sprites into an image instead of loading a PNG.
local sheet = new_image(128, 32)
set_target(sheet)
cls(rgb(0, 0, 0, 0))
fill_circle(16, 16, 14, rgb(255, 200, 50))        -- sun
fill_rect(36, 10, 20, 14, rgb(90, 160, 255))        -- ship body
fill_triangle(56, 10, 64, 17, 56, 24, rgb(90, 160, 255))
fill_rect(72, 6, 20, 20, rgb(100, 220, 100))        -- crate
rect(72, 6, 20, 20, rgb(30, 90, 30))
for i = 0, 3 do fill_rect(100 + i * 6, 4 + i * 6, 6, 6, hsv(i * 60, 1, 1)) end -- stairs
set_target()

local things = {}
for i = 1, 40 do
  things[i] = { x = random() * w, y = random() * h, a = random() * 6.28, s = 1 + random() * 2, spr = random(0, 3), spin = (random() - 0.5) * 0.1 }
end

while true do
  cls(rgb(16, 8, 24))
  for _, t in ipairs(things) do
    t.a = t.a + t.spin
    t.y = t.y + t.s * 0.5
    if t.y > h + 32 then t.y = -32 end
    draw_sprite(sheet, t.spr * 32, 0, 32, 32, t.x, t.y, { rotation = t.a, scale = t.s, origin_x = 16, origin_y = 16 })
  end
  draw_image(sheet, 8, h - 40, { scale = 1 })
  draw_text(8, h - 56, "the sheet:", rgb(200, 200, 200))
  flip()
end
