-- README: how to use Argon, as a program.
cls()
local lines = {
  "ARGON DEMOS",
  "",
  "  RUN hello       text input and output",
  "  RUN quadratic   a homework solver",
  "  RUN bounce      the main loop and pad buttons",
  "  RUN shapes      every drawing primitive",
  "  RUN plot        graph a function",
  "  RUN clock       lines and line_width",
  "  RUN starfield   pixels",
  "  RUN particles   20,000 pixels at 60 fps",
  "  RUN sprites     set_target and draw_sprite",
  "  RUN snake       a game on the text grid, with save/load",
  "  RUN retro       a 320x240 cart with a camera",
  "  RUN pong        the classic",
  "  RUN matrix      digital rain (alt_screen)",
  "  RUN beeps       the synth, key by key",
  "  RUN chiptune    a three-channel tune",
  "",
  "DEMOS is read-only. To make your own version:",
  "  COPY DEMOS/pong mypong      then  EDIT mypong",
  "",
  "Press any key.",
}
for i, l in ipairs(lines) do
  text_color(i == 1 and rgb(255, 220, 80) or rgb(255, 255, 255))
  print(l)
end
wait_key()
