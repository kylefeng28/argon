-- chiptune: a three-channel tune on the synth, with a visualizer.
-- Channels: 1 lead (square), 2 bass (triangle), drums use beep() noise.
alt_screen()
local w, h = screen_size()
local bpm = 150
local step_time = 60 / bpm / 2 -- eighth notes

-- Patterns are 16 steps; "-" holds the previous note, "." is a rest.
local lead = { "E5", "-", "B4", "C5", "D5", "-", "C5", "B4", "A4", "-", "A4", "C5", "E5", "-", "D5", "C5",
               "B4", "-", "-", "C5", "D5", "-", "E5", "-", "C5", "-", "A4", "-", "A4", "-", ".", "." }
local bass = { "E2", ".", "E2", ".", "A2", ".", "A2", ".", "D2", ".", "D2", ".", "G2", ".", "G2", ".",
               "E2", ".", "E2", ".", "A2", ".", "A2", ".", "D2", ".", "G2", ".", "E2", ".", ".", "." }
local drums = { "k", ".", "h", ".", "s", ".", "h", ".", "k", ".", "h", "k", "s", ".", "h", "h",
                "k", ".", "h", ".", "s", ".", "h", ".", "k", ".", "h", "k", "s", "s", "h", "s" }

local step, clock = 0, 0
local bars = {}
for i = 1, 32 do bars[i] = 0 end
local playing = true
local lead_hz, bass_hz = 0, 0

local function hit(kind)
  if kind == "k" then beep(90, 0.08, { wave = "triangle", slide = -600, volume = 0.9 })
  elseif kind == "s" then beep(2500, 0.08, { wave = "noise", volume = 0.5, release = 0.06 })
  elseif kind == "h" then beep(8000, 0.02, { wave = "noise", volume = 0.25 }) end
end

local function play_step(i)
  local l = lead[i]
  if l == "." then tone(1)
  elseif l ~= "-" then lead_hz = note_freq(l) tone(1, { freq = lead_hz, wave = "square", duty = 0.3, volume = 0.35 }) end
  local b = bass[i]
  if b == "." then tone(2)
  elseif b ~= "-" then bass_hz = note_freq(b) tone(2, { freq = bass_hz, wave = "triangle", volume = 0.6 }) end
  if drums[i] ~= "." then hit(drums[i]) end
  bars[i] = 1
end

while true do
  if button_pressed("a") then
    playing = not playing
    if not playing then tone(1) tone(2) end
  end
  if button_pressed("up") then bpm = math.min(bpm + 10, 300) step_time = 60 / bpm / 2 end
  if button_pressed("down") then bpm = math.max(bpm - 10, 60) step_time = 60 / bpm / 2 end

  if playing then
    clock = clock + delta_time()
    while clock >= step_time do
      clock = clock - step_time
      step = step % #lead + 1
      play_step(step)
    end
  end

  cls(rgb(8, 8, 20))
  for i = 1, 32 do
    bars[i] = bars[i] * 0.9
    local x = (i - 1) * (w / 32)
    local col = (i == step) and rgb(255, 255, 255) or hsv(i * 11, 0.7, 0.9)
    fill_rect(x + 2, h - 40 - bars[i] * 120, w / 32 - 4, bars[i] * 120 + 4, col)
  end
  -- Waveform sketch of the two tones.
  for x = 0, w - 1 do
    local t = x / w * 0.02
    local y1 = (math.sin(t * lead_hz * 6.28) > 0) and 1 or -1
    local y2 = math.sin(t * bass_hz * 6.28)
    pixel(x, h / 2 - 60 + y1 * 20, rgb(255, 200, 80))
    pixel(x, h / 2 + 20 + y2 * 25, rgb(120, 200, 255))
  end
  draw_text(8, 8, "CHIPTUNE", rgb(255, 255, 255), 2)
  draw_text(8, 44, string.format("%d bpm   Z play/pause   UP/DOWN tempo   step %02d", bpm, step), rgb(160, 160, 200))
  flip()
end
