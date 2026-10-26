-- hud: a tactical readout in the style of a certain cyberpunk anime's
-- field terminals. Everything is fake data; it just looks busy.
-- argon: filter = "crt"
alt_screen()
local w, h = screen_size()
local G = rgb(40, 255, 170)         -- phosphor green
local G2 = rgb(20, 150, 100)
local G3 = rgb(8, 60, 44)
local BG = rgb(2, 10, 8)

local function panel(x, y, pw, ph, title)
  fill_rect(x, y, pw, ph, BG)
  rect(x, y, pw, ph, G2)
  line(x + 1, y + 1, x + 1, y + ph - 2, G2)
  -- corner ticks
  for _, c in ipairs({ { x, y }, { x + pw - 1, y }, { x, y + ph - 1 }, { x + pw - 1, y + ph - 1 } }) do
    fill_rect(c[1] - 1, c[2] - 1, 3, 3, G)
  end
  if title then
    fill_rect(x + 6, y - 1, #title * 8 + 4, 3, BG)
    draw_text(x + 8, y - 8, title, G)
  end
end

local hex = {}
for i = 1, 12 do hex[i] = string.format("%08X", random(0, 0xFFFFFFF)) end
local signal = {}
for i = 1, 48 do signal[i] = random() end
local log = { "LINK ESTABLISHED", "HANDSHAKE OK", "SCANNING SECTOR 7", "GHOST SIGNATURE: NONE" }
local lines = { "BARRIER MAZE", "ATTACK BARRIER", "DECOY", "PUPPET MASTER", "SECTION 9", "THERMOPTIC", "CYBERBRAIN", "NET DIVE" }

while true do
  local t = time()
  cls(BG)

  -- Reticle, top left.
  panel(10, 14, 190, 190, "TARGET")
  local cx, cy = 105, 109
  for i = 1, 3 do circle(cx, cy, 20 * i + 8, i == 2 and G or G2) end
  for k = 0, 23 do
    local a = k / 24 * math.pi * 2 + t * 0.4
    local r1, r2 = 72, (k % 6 == 0) and 84 or 78
    line(cx + math.cos(a) * r1, cy + math.sin(a) * r1, cx + math.cos(a) * r2, cy + math.sin(a) * r2, G)
  end
  for k = 0, 2 do
    local a = t * 1.5 + k * 2.094
    local pts = {}
    for j = 0, 8 do
      local b = a + j / 8 * 1.2
      pts[#pts + 1] = cx + math.cos(b) * 50
      pts[#pts + 1] = cy + math.sin(b) * 50
    end
    for j = 1, #pts - 3, 2 do line(pts[j], pts[j + 1], pts[j + 2], pts[j + 3], G) end
  end
  local sa = t * 0.9
  line(cx, cy, cx + math.cos(sa) * 88, cy + math.sin(sa) * 88, G)
  fill_circle(cx + math.cos(t * 0.7) * 30, cy + math.sin(t * 1.1) * 25, 3, rgb(255, 120, 60))
  draw_text(20, 184, string.format("AZ %05.1f  EL %+05.1f", (sa * 57.3) % 360, math.sin(t) * 30), G2)

  -- Hex dump, bottom left.
  panel(10, 226, 190, 160, "MEMORY")
  if frame() % 6 == 0 then
    table.remove(hex, 1)
    hex[#hex + 1] = string.format("%08X", random(0, 0xFFFFFFF))
  end
  for i, hx in ipairs(hex) do
    draw_text(18, 232 + (i - 1) * 12, string.format("%04X %s", 0x4A00 + i * 16, hx), i == #hex and G or G2)
  end

  -- Signal bars, top right.
  panel(214, 14, 416, 120, "SIGNAL")
  for i = 1, 48 do
    signal[i] = clamp(signal[i] + (random() - 0.5) * 0.15, 0.05, 1)
    local bx = 222 + (i - 1) * 8.5
    local bh = signal[i] * 92
    fill_rect(bx, 124 - bh, 6, bh, signal[i] > 0.8 and rgb(255, 140, 60) or G)
    pixel(bx + 3, 124 - bh - 3, G)
  end
  for y = 40, 124, 21 do line(216, y, 628, y, G3) end

  -- Waveform, middle right.
  panel(214, 150, 416, 90, "CARRIER")
  for x = 0, 410 do
    local v = math.sin(x * 0.05 + t * 4) * math.sin(x * 0.011 + t) * 30
    pixel(218 + x, 195 + v, G)
    if x % 40 == 0 then line(218 + x, 160, 218 + x, 236, G3) end
  end
  line(218, 195, 628, 195, G3)

  -- Status table, bottom right.
  panel(214, 256, 416, 130, "PROCESSES")
  for i, name in ipairs(lines) do
    local y = 262 + (i - 1) * 15
    local load = (math.sin(t * 0.7 + i) * 0.5 + 0.5)
    draw_text(222, y, string.format("%02d", i), G2)
    draw_text(246, y, name, G)
    fill_rect(400, y + 4, 180, 7, G3)
    fill_rect(400, y + 4, 180 * load, 7, load > 0.85 and rgb(255, 140, 60) or G)
    draw_text(588, y, string.format("%3d%%", math.floor(load * 100)), G2)
  end

  -- Footer ticker.
  if frame() % 150 == 0 then
    table.insert(log, string.format("PACKET %04X ROUTED", random(0, 0xFFFF)))
    if #log > 4 then table.remove(log, 1) end
  end
  draw_text(10, h - 12, table.concat(log, "   //   "), G2)
  if frame() % 60 < 30 then draw_text(w - 60, h - 12, "ONLINE", G) end
  flip()
end
