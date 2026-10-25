-- hello: text input and output
cls()
print("Hello! I am Argon.")
print()
local name = input("What is your name? ")
if name == "" then name = "stranger" end
print()
text_color(rgb(255, 220, 80))
print("Nice to meet you, " .. name .. "!")
text_color(rgb(255, 255, 255))
print()
print("Press any key to return to the shell.")
wait_key()
