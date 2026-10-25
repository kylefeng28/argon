-- chiptune: Korobeiniki (the Tetris theme) on the synth.
-- Four tracks: lead (ch 1), harmony arpeggio (ch 3), bass (ch 2), drums (beeps).
-- A step is an eighth note; every pattern below is 16 steps (two bars).
alt_screen()
local w, h = screen_size()
local bpm = 150
local step_time = 60 / bpm / 2

-- Lead melody. "-" holds the previous note, "." is a rest.
local lead_patterns = {
  "E5 - B4 C5 D5 - C5 B4   A4 - A4 C5 E5 - D5 C5",
  "B4 - - C5 D5 - E5 -     C5 - A4 - A4 - . .",
  ". D5 - F5 A5 - G5 F5    E5 - - C5 E5 - D5 C5",
  "B4 - B4 C5 D5 - E5 -    C5 - A4 - A4 - . .",
  "E5 - - - C5 - - -       D5 - - - B4 - - -",
  "C5 - - - A4 - - -       G#4 - - - B4 - . .",
  "E5 - - - C5 - - -       D5 - - - B4 - - -",
  "C5 - E5 - A5 - - -      G#5 - - - - - . .",
}
-- One chord per bar (root, third, fifth, octave for the arpeggio).
local chords = {
  { "E", "G", "B" }, { "A", "C", "E" }, { "G", "B", "D" }, { "A", "C", "E" },
  { "D", "F", "A" }, { "C", "E", "G" }, { "G", "B", "D" }, { "A", "C", "E" },
  { "A", "C", "E" }, { "E", "G#", "B" }, { "A", "C", "E" }, { "E", "G#", "B" },
  { "A", "C", "E" }, { "E", "G#", "B" }, { "A", "C", "E" }, { "E", "G#", "B" },
}
local drum_pattern = "k h s h k h s h  k h s h k h s s"

local function split(s)
  local t = {}
  for tok in s:gmatch("%S+") do t[#t + 1] = tok end
  return t
end

-- Flatten patterns into one long song (128 steps = 16 bars).
local lead = {}
for _, p in ipairs(lead_patterns) do
  for _, n in ipairs(split(p)) do lead[#lead + 1] = n end
end
local bass, harmony, drums = {}, {}, {}
local drum_steps = split(drum_pattern)
for bar = 1, #chords do
  local c = chords[bar]
  -- Bass: root / octave / fifth / octave, pumping on every eighth.
  local b = { c[1] .. "2", c[1] .. "3", c[3] .. "2", c[1] .. "3", c[1] .. "2", c[1] .. "3", c[3] .. "2", c[1] .. "3" }
  -- Harmony: a quiet arpeggio of the chord one octave below the lead.
  local a = { c[1] .. "4", c[2] .. "4", c[3] .. "4", c[2] .. "4", c[1] .. "4", c[2] .. "4", c[3] .. "4", c[2] .. "4" }
  for i = 1, 8 do
    bass[#bass + 1] = b[i]
    harmony[#harmony + 1] = a[i]
    drums[#drums + 1] = drum_steps[(bar - 1) % 2 * 8 + i]
  end
end
local total = #lead

local function drum(kind)
  if kind == "k" then beep(100, 0.07, { wave = "triangle", slide = -800, volume = 0.9 })
  elseif kind == "s" then beep(2200, 0.07, { wave = "noise", volume = 0.45, release = 0.05 })
  elseif kind == "h" then beep(9000, 0.015, { wave = "noise", volume = 0.2 }) end
end

local playing, muted = true, { false, false, false, false }
local step, clock = 0, 0
local lead_hz, bass_hz, harm_hz = 0, 0, 0
local lead_name, bass_name, harm_name = "", "", ""

local function play_note(channel, name, opts, mute)
  if name == "." then
    tone(channel)
    return 0, ""
  elseif name == "-" then
    return nil
  end
  local hz = note_freq(name)
  if mute then tone(channel) else
    opts.freq = hz
    tone(channel, opts)
  end
  return hz, name
end

local function play_step(i)
  local hz, nm = play_note(1, lead[i], { wave = "square", duty = 0.25, volume = 0.3, release = 0.03 }, muted[1])
  if hz then lead_hz, lead_name = hz, nm end
  hz, nm = play_note(3, harmony[i], { wave = "square", duty = 0.5, volume = 0.1 }, muted[2])
  if hz then harm_hz, harm_name = hz, nm end
  hz, nm = play_note(2, bass[i], { wave = "triangle", volume = 0.6 }, muted[3])
  if hz then bass_hz, bass_name = hz, nm end
  if drums[i] ~= "." and not muted[4] then drum(drums[i]) end
end

local function silence() for c = 1, 3 do tone(c) end end

while true do
  if button_pressed("a") then playing = not playing if not playing then silence() end end
  if button_pressed("up") then bpm = math.min(bpm + 10, 300) step_time = 60 / bpm / 2 end
  if button_pressed("down") then bpm = math.max(bpm - 10, 60) step_time = 60 / bpm / 2 end
  for i = 1, 4 do
    if key_pressed(tostring(i)) then muted[i] = not muted[i] end
  end
  if key_pressed("r") then step, clock = 0, 0 silence() end

  if playing then
    clock = clock + delta_time()
    while clock >= step_time do
      clock = clock - step_time
      step = step % total + 1
      play_step(step)
    end
  end

  -- Visualizer: a piano roll of the next two bars for each track.
  cls(rgb(8, 8, 20))
  local ts = h // 400
  draw_text(8, 8, "KOROBEINIKI", rgb(255, 255, 255), 2 * ts)
  draw_text(8, 44 * ts, string.format("%d bpm  bar %2d/16  Z play/pause  UP/DOWN tempo  1-4 mute tracks  R restart", bpm, (step - 1) // 8 + 1), rgb(150, 150, 190))

  -- name, notes, color, lowest semitone shown (relative to A2), semitone span
  local tracks = {
    { "lead", lead, rgb(255, 200, 80), 22, 18 },
    { "harmony", harmony, rgb(180, 120, 255), 8, 22 },
    { "bass", bass, rgb(120, 200, 255), -14, 28 },
  }
  local ts = h // 400
  local roll_x, roll_w, cell = 100 * ts, w - 110 * ts, (w - 110 * ts) / 32
  for ti, t in ipairs(tracks) do
    local y0 = (80 + (ti - 1) * 90) * ts
    local col = muted[ti] and rgb(70, 70, 80) or t[3]
    draw_text(8, y0 + 30 * ts, t[1], col)
    fill_rect(roll_x, y0, roll_w, 80 * ts, rgb(16, 16, 36))
    local last_hz
    for k = 0, 31 do
      local i = (step - 1 + k) % total + 1
      local n = t[2][i]
      local hz
      if n == "." then hz = nil last_hz = nil
      elseif n == "-" then hz = last_hz
      else hz = note_freq(n) last_hz = hz end
      if hz then
        -- Map pitch to height within the track's range.
        local semis = 12 * math.log(hz / 110, 2)
        local y = y0 + (76 - clamp((semis - t[4]) / t[5], 0, 1) * 72) * ts
        local bright = (k == 0) and rgb(255, 255, 255) or col
        fill_rect(roll_x + k * cell, y, cell - 1, 4 * ts, bright)
      end
    end
    line(roll_x, y0, roll_x, y0 + 80 * ts, rgb(255, 255, 255))
  end
  -- Drums row.
  local y0 = (80 + 3 * 90) * ts
  draw_text(8, y0 + 2, "drums", muted[4] and rgb(70, 70, 80) or rgb(255, 120, 120))
  for k = 0, 31 do
    local i = (step - 1 + k) % total + 1
    local d = drums[i]
    if d ~= "." then
      local c = d == "k" and rgb(255, 100, 100) or d == "s" and rgb(255, 180, 120) or rgb(200, 200, 200)
      local size = (d == "k" and 10 or d == "s" and 7 or 4) * ts
      fill_rect(roll_x + k * cell, y0 + 12 * ts - size / 2, cell - 1, size, muted[4] and rgb(70, 70, 80) or c)
    end
  end
  draw_text(roll_x, h - 18 * ts, string.format("lead %-4s  harmony %-4s  bass %-3s", lead_name, harm_name, bass_name), rgb(120, 120, 160))
  flip()
end
