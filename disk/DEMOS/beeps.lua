-- beeps: the synth, one sound at a time. Press keys to hear them.
alt_screen()
cls()
print("BEEPS - press a key:")
print()
print("  1  square      2  triangle    3  saw")
print("  4  sine        5  noise       6  laser (slide)")
print("  7  scale       8  chord       9  alarm")
print()
print("  Esc quits")
print()

local notes = { "C4", "D4", "E4", "F4", "G4", "A4", "B4", "C5" }
while true do
  local k = read_key()
  if k == "1" then beep(440, 0.3, { wave = "square" })
  elseif k == "2" then beep(440, 0.3, { wave = "triangle" })
  elseif k == "3" then beep(440, 0.3, { wave = "saw", volume = 0.3 })
  elseif k == "4" then beep(440, 0.3, { wave = "sine" })
  elseif k == "5" then beep(1000, 0.3, { wave = "noise", volume = 0.4 })
  elseif k == "6" then beep(1800, 0.25, { wave = "square", slide = -5000, duty = 0.2 })
  elseif k == "7" then
    for _, n in ipairs(notes) do beep(note_freq(n), 0.12, { wave = "square", duty = 0.4 }) wait(0.13) end
  elseif k == "8" then
    beep(note_freq("C4"), 0.6, { wave = "triangle" })
    beep(note_freq("E4"), 0.6, { wave = "triangle" })
    beep(note_freq("G4"), 0.6, { wave = "triangle" })
  elseif k == "9" then
    for i = 1, 4 do beep(880, 0.1) wait(0.12) beep(660, 0.1) wait(0.12) end
  end
  if k then print("  " .. k) end
  flip()
end
