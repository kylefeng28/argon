- Drop a Shadertoy-style fragment shader into disk/DEMOS/shadertoy as name.glsl.
  RUN name runs it; the console wraps it in a generated full-screen Lua program
  (alt_screen, load_shader, draw_shader + flip loop; the mouse feeds iMouse).
- Header settings work with // comments: // argon: resolution = "hd".
  argon check compiles .glsl files with naga.
- Also support NEW/EDIT name.glsl to create/edit shader templates; the editor highlights GLSL
  (// and /* */ comments, keywords, Shadertoy and GLSL builtins, #directives)
  based on the file extension, and { opens an indented block.
- DEMOS/shadertoy/README.glsl documents the credit
  header convention (title, Source URL, Author) and porting notes, and is
  itself a runnable example.
- The smoke test covers .glsl demos.

What else came with it:

- NEW name.glsl creates a shader from a template; EDIT opens .glsl files with GLSL highlighting (comments, keywords, Shadertoy and GLSL builtins, #defines), and F5 runs
them like any program. The screenshots above show the editor and the running README shader.
- Header settings work with // comments: // argon: resolution = "hd" or fps = 30.
- argon check name compiles .glsl files through naga, and the headless smoke test now covers everything under DEMOS/shadertoy/.
- DEMOS/shadertoy/README.glsl is itself a runnable example and documents the credit convention I'd suggest for the files you add:
- Compile errors still point at the file's own line numbers because the source is embedded unchanged.

**Caveats**
Shadertoy shaders that use only mainImage and the standard uniforms should run as-is or with a small edit if naga rejects an ES-ism; the error names the line.

Some Shadertoy shaders reference iChannel textures or multiple Buffer passes, and .glsl programs have black channels, so if a port needs textures it becomes a short .lua wrapper that builds an image and passes it as a
channel; multipass (Buffer A/B) isn't supported yet.


Template:
```
// myshader.glsl
// Title of the shader
// Source: https://www.shadertoy.com/view/XXXXXX
// Author: username (license as stated on Shadertoy)
```

---

Shadertoy examples:
- Raymarching generators: https://www.shadertoy.com/view/Xtf3Rn
- Ocean: https://www.shadertoy.com/view/MdXyzX
- Fractal pyramid: https://www.shadertoy.com/view/tsXBzS
- Cyber fuji: https://www.shadertoy.com/view/Wt33Wf
- Mandelbulb: https://www.shadertoy.com/view/MdXSWn
- Particle experimnet: https://www.shadertoy.com/view/MddGWN

