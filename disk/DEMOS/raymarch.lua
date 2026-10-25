-- raymarch: a Shadertoy-style signed-distance scene. The camera orbits;
-- drag the mouse to look around, wheel to zoom. This is plain Shadertoy
-- GLSL: paste your own mainImage into load_shader and it just runs.
alt_screen()
local w, h = screen_size()

local sh = load_shader([[
uniform float zoom;

float sdBox(vec3 p, vec3 b) { vec3 q = abs(p) - b; return length(max(q, 0.0)) + min(max(q.x, max(q.y, q.z)), 0.0); }
float sdTorus(vec3 p, vec2 t) { vec2 q = vec2(length(p.xz) - t.x, p.y); return length(q) - t.y; }

vec2 map(vec3 p) {
    // floor grid
    float floorD = p.y + 1.0;
    // spinning torus
    float a = iTime * 0.6;
    vec3 q = p; q.xz = mat2(cos(a), -sin(a), sin(a), cos(a)) * q.xz;
    float torus = sdTorus(q.xzy, vec2(1.2, 0.25));
    // orbiting cubes
    float cubes = 1e9;
    for (int i = 0; i < 5; i++) {
        float t = iTime * 0.8 + float(i) * 1.2566;
        vec3 c = p - vec3(cos(t) * 2.4, sin(t * 2.0) * 0.4, sin(t) * 2.4);
        cubes = min(cubes, sdBox(c, vec3(0.25)));
    }
    float d = min(floorD, min(torus, cubes));
    float id = d == floorD ? 0.0 : (d == torus ? 1.0 : 2.0);
    return vec2(d, id);
}

vec3 normal(vec3 p) {
    vec2 e = vec2(0.001, 0.0);
    return normalize(vec3(map(p + e.xyy).x - map(p - e.xyy).x, map(p + e.yxy).x - map(p - e.yxy).x, map(p + e.yyx).x - map(p - e.yyx).x));
}

void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = (fragCoord - 0.5 * iResolution.xy) / iResolution.y;
    // Camera: orbit, mouse drag adds yaw/pitch.
    vec2 m = iMouse.xy / iResolution.xy;
    float yaw = iTime * 0.3 + (m.x - 0.5) * 6.28;
    // Mouse up tilts the view up (pulls the camera down toward the floor).
    float pitch = 0.35 - (m.y - 0.5) * 1.5;
    vec3 ro = vec3(cos(yaw) * cos(pitch), sin(pitch), sin(yaw) * cos(pitch)) * (6.0 / max(zoom, 0.1));
    vec3 ta = vec3(0.0, 0.0, 0.0);
    vec3 f = normalize(ta - ro), r = normalize(cross(f, vec3(0.0, 1.0, 0.0))), u = cross(r, f);
    vec3 rd = normalize(uv.x * r + uv.y * u + 1.6 * f);

    float t = 0.0; float id = -1.0;
    for (int i = 0; i < 128; i++) {
        vec2 h = map(ro + rd * t);
        if (h.x < 0.001) { id = h.y; break; }
        t += h.x;
        if (t > 40.0) break;
    }
    vec3 col = vec3(0.01, 0.02, 0.05);
    if (id >= 0.0) {
        vec3 p = ro + rd * t;
        vec3 n = normal(p);
        vec3 ldir = normalize(vec3(0.6, 0.9, -0.4));
        float dif = clamp(dot(n, ldir), 0.0, 1.0);
        float spe = pow(clamp(dot(reflect(-ldir, n), -rd), 0.0, 1.0), 32.0);
        float rim = pow(1.0 - clamp(dot(n, -rd), 0.0, 1.0), 3.0);
        vec3 base;
        if (id == 0.0) {
            // Grid floor: glowing cyan lines on dark tiles.
            vec2 g = abs(fract(p.xz) - 0.5);
            float line = smoothstep(0.47, 0.5, max(g.x, g.y));
            base = mix(vec3(0.02, 0.04, 0.08), vec3(0.0, 0.9, 1.0), line);
            dif = 0.5 + 0.5 * dif;
        } else if (id == 1.0) {
            base = vec3(0.0, 0.85, 1.0);
        } else {
            base = vec3(1.0, 0.5, 0.1);
        }
        col = base * (0.15 + 0.85 * dif) + vec3(1.0) * spe * 0.6 + base * rim * 1.5;
        // soft shadow toward the light
        float sh = 1.0, st = 0.05;
        for (int i = 0; i < 24; i++) { float d = map(p + ldir * st).x; sh = min(sh, 8.0 * d / st); st += clamp(d, 0.02, 0.3); if (sh < 0.01) break; }
        col *= 0.4 + 0.6 * clamp(sh, 0.0, 1.0);
        col = mix(col, vec3(0.01, 0.02, 0.05), 1.0 - exp(-0.02 * t * t));
    }
    fragColor = vec4(pow(col, vec3(0.8)), 1.0);
}
]])

local zoom = 1
while true do
  zoom = clamp(zoom * 1.1 ^ mouse_wheel(), 0.4, 3)
  draw_shader(sh, { uniforms = { zoom = zoom } })
  local th = 16 * (h // 400) + 4
  fill_rect(0, h - th, w, th, rgb(0, 0, 0, 160))
  draw_text(4, h - th + 2, string.format("RAYMARCH  %.0f fps   drag to look  wheel zoom", 1 / math.max(delta_time(), 1e-4)), rgb(150, 220, 255))
  flip()
end
