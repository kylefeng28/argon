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
  "  RUN chiptune    Korobeiniki (Tetris) on four tracks",
  "  RUN cube        a 3D cube, software rendered",
  "  RUN lightcycles light cycles vs the program",
  "  RUN mandelbrot  progressive Mandelbrot at 320x240",
  "  RUN julia       an animated Julia set",
  "  RUN hud         a tactical readout (crt filter)",
  "  RUN life        Conway's Game of Life with afterglow",
  "  RUN flock       boids with light trails",
  "  RUN attractor   strange attractors in 3D",
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
