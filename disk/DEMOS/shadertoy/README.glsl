// DEMOS/shadertoy: Shadertoy shaders that run as programs.
//
// Any .glsl file on the disk is a program: RUN name wraps it in a
// full-screen draw loop (alt_screen, load_shader, draw_shader, flip), the
// mouse feeds iMouse, Esc stops it. No Lua needed.
//
// Credit the author at the top of each file, like this:
//
//   // Title of the shader
//   // Source: https://www.shadertoy.com/view/XXXXXX
//   // Author: username (license as stated on Shadertoy)
//
// Optional settings go in the header too:
//   // argon: resolution = "hd"      (tiny, retro, classic, hd, WxH)
//   // argon: fps = 30
//
// Notes for porting:
// - fragCoord is y-up and texture() reads channels bottom-up, as on Shadertoy.
// - iChannel0-3 are black unless a Lua program feeds them images; shaders
//   that depend on textures or Buffer passes need a .lua wrapper instead.
// - naga compiles GLSL 450, so a few ES-isms may need touching up (the error
//   message names the line).
// argon: resolution = "classic"

void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = (fragCoord - 0.5 * iResolution.xy) / iResolution.y;
    float d = length(uv);
    vec3 col = 0.5 + 0.5 * cos(iTime * 0.6 + d * 12.0 + vec3(0.0, 2.0, 4.0));
    col *= smoothstep(1.0, 0.2, d);
    fragColor = vec4(col * vec3(0.3, 0.9, 1.0), 1.0);
}
