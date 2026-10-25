-- matrix: digital rain on the text grid. Runs on the alternate screen, so
-- the shell is exactly as you left it when you press Esc.
-- argon: fps = 20
alt_screen()
local cols, rows = text_size()
local glyphs = "0123456789ABCDEFZ:.=*+-<>|^ΣΩψΔ¥§±÷¢£µ¶"
local gl = {}
for _, cp in utf8.codes(glyphs) do gl[#gl + 1] = utf8.char(cp) end

local drops = {}
for c = 0, cols - 1 do
  drops[c] = { y = random(-rows, 0), speed = random(1, 2), len = random(6, rows - 4) }
end

local bright = rgb(200, 255, 200)
local shades = {}
for i = 1, 24 do shades[i] = rgb(0, math.floor(255 * (1 - (i - 1) / 24)), math.floor(60 * (1 - (i - 1) / 24))) end

cls(rgb(0, 0, 0))
while true do
  -- Fade: redraw every column's trail with dimmer shades.
  for c = 0, cols - 1 do
    local d = drops[c]
    if frame() % d.speed == 0 then d.y = d.y + 1 end
    for i = 0, d.len do
      local y = d.y - i
      if y >= 0 and y < rows then
        if i == 0 then
          put_char(c, y, gl[random(1, #gl)], bright)
        elseif i == d.len then
          put_char(c, y, " ")
        else
          local ch = select(1, get_char(c, y)) or " "
          if ch == " " or random() < 0.05 then ch = gl[random(1, #gl)] end
          put_char(c, y, ch, shades[math.min(#shades, math.floor(i / d.len * #shades) + 1)])
        end
      end
    end
    if d.y - d.len > rows then
      d.y, d.speed, d.len = random(-rows, -1), random(1, 2), random(6, rows - 4)
    end
  end
  flip()
end
