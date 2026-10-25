-- README: how to use Argon, as a program.
cls()
local lines = {
  "ARGON DEMOS",
  "",
  "  RUN hello       text input and output",
  "  RUN snake       a game on the text grid, with save/load",
  "",
  "DEMOS is read-only. To make your own version:",
  "  COPY DEMOS/snake mysnake      then  EDIT snake",
  "",
  "Press any key.",
}
for i, l in ipairs(lines) do
  text_color(i == 1 and rgb(255, 220, 80) or rgb(255, 255, 255))
  print(l)
end
wait_key()
