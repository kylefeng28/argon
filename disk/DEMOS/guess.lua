-- guess: guess a number
-- The computer thinks of a number from 1 to 100. You find it.
cls(rgb(7, 7, 11))

local n = random(1, 100)
local tries = 0

text_color(rgb(232, 230, 240))
print("  I am thinking of a number from 1 to 100.")
print()

while true do
  local g = tonumber(input("  Your guess? "))
  if g == nil then
    print("  That is not a number.")
  else
    tries = tries + 1
    if g == n then
      text_color(rgb(169, 139, 255))
      print("  Yes! It was " .. n .. ". You took " .. tries .. " tries.")
      break
    elseif g < n then
      print("  Higher.")
    else
      print("  Lower.")
    end
  end
end

text_color(rgb(108, 104, 128))
print()
print("  Press any key.")
wait_key()
