-- particles_gpu: one million particles simulated and drawn on the GPU.
-- A compute shader moves them through a drifting flow field; draw_points
-- renders them straight from the buffer, so Lua never touches a particle.
-- Hold the left button to attract, right to repel. Wheel: flow strength.
-- Z: reseed. X: trails on/off.
alt_screen()
local w, h = screen_size()
local n = 1000000

local particles = new_buffer(n, "vec4")   -- xy = position (px), zw = velocity
local colors = new_buffer(n, "vec4")      -- rgba per particle, written by the sim

local seed = load_compute([[
layout(local_size_x = 256) in;
layout(std430, binding = 0) buffer P { vec4 p[]; };
uniform vec2 size;
uniform float salt;

float hash(uint x) {
    x ^= x >> 16; x *= 0x7feb352dU; x ^= x >> 15; x *= 0x846ca68bU; x ^= x >> 16;
    return float(x) / 4294967295.0;
}

void main() {
    uint i = gl_GlobalInvocationID.x;
    if (i >= iCount.x) return;
    uint s = i * 4u + uint(salt);
    // A ring of particles around the center, slowly orbiting.
    float a = hash(s) * 6.2831853;
    float r = (0.15 + 0.3 * sqrt(hash(s + 1u))) * size.y;
    vec2 c = size * 0.5;
    vec2 pos = c + vec2(cos(a), sin(a)) * r;
    vec2 vel = vec2(-sin(a), cos(a)) * (40.0 + 40.0 * hash(s + 2u));
    p[i] = vec4(pos, vel);
}
]])

local step = load_compute([[
layout(local_size_x = 256) in;
layout(std430, binding = 0) buffer P { vec4 p[]; };
layout(std430, binding = 1) buffer C { vec4 col[]; };
uniform vec2 size;
uniform vec2 target;
uniform float pull;     // +1 attract, -1 repel, 0 none
uniform float flow;
uniform float dt;

vec3 hsv2rgb(vec3 c) {
    vec3 k = clamp(abs(mod(c.x * 6.0 + vec3(0.0, 4.0, 2.0), 6.0) - 3.0) - 1.0, 0.0, 1.0);
    return c.z * mix(vec3(1.0), k, c.y);
}

void main() {
    uint i = gl_GlobalInvocationID.x;
    if (i >= iCount.x) return;
    vec2 pos = p[i].xy;
    vec2 vel = p[i].zw;

    // A smooth, slowly changing flow field (sums of sines, cheap curl-ish).
    vec2 q = pos / size.y * 3.0;
    float t = iTime * 0.25;
    vec2 f = vec2(sin(q.y * 1.7 + t) + sin(q.y * 3.1 - t * 1.3) * 0.5,
                  cos(q.x * 1.5 - t) + cos(q.x * 2.9 + t * 1.1) * 0.5);
    vel += f * flow * 60.0 * dt;

    if (pull != 0.0) {
        vec2 d = target - pos;
        float l = length(d) + 20.0;
        vel += d / l * pull * 2400.0 * dt * (size.y / l) * 0.25;
    }
    vel *= exp(-0.6 * dt);
    pos += vel * dt;

    // Wrap around the edges.
    pos = mod(pos + size, size);
    p[i] = vec4(pos, vel);

    // Color by speed: deep blue -> cyan -> white -> orange at the fastest.
    float sp = length(vel) / (size.y * 0.5);
    vec3 c = hsv2rgb(vec3(0.58 - 0.5 * clamp(sp, 0.0, 1.0), 0.85 - 0.4 * smoothstep(0.4, 0.8, sp), 1.0));
    // Faint on their own: with ~1 particle per pixel, brightness should
    // come from where they bunch up, not from each one.
    col[i] = vec4(c, 0.025 + 0.06 * clamp(sp, 0.0, 1.0));
}
]])

local function reseed(salt)
  dispatch(seed, n, { buffers = { particles }, uniforms = { size = { w, h }, salt = salt } })
end
reseed(0)

local flow, trails, salt = 1.0, true, 0
cls(rgb(0, 0, 0))

while true do
  local mx, my = mouse()
  local pull = 0
  if mouse_button("left") then pull = 1 elseif mouse_button("right") then pull = -1 end
  flow = clamp(flow * 1.15 ^ mouse_wheel(), 0.05, 8)
  if button_pressed("a") then salt = salt + 7919 reseed(salt) end
  if button_pressed("b") then trails = not trails end

  dispatch(step, n, {
    buffers = { particles, colors },
    uniforms = { size = { w, h }, target = { mx, my }, pull = pull, flow = flow, dt = math.min(delta_time(), 1 / 30) },
  })

  -- Trails: fade what is there instead of clearing it.
  if trails then
    fill_rect(0, 0, w, h, rgb(0, 0, 0, 70))
  else
    cls(rgb(0, 0, 0))
  end
  blend("add")
  draw_points(particles, { colors = colors, size = 1 })
  blend("alpha")

  local ts = h // 400
  fill_rect(0, 0, w, 18 * ts, rgb(0, 0, 0, 180))
  draw_text(4, 0, string.format("1,000,000 PARTICLES  %3.0f fps  flow %.2f   hold mouse: attract/repel  wheel: flow  Z reseed  X trails",
    1 / math.max(delta_time(), 1e-4), flow), rgb(150, 220, 255))
  if mouse_button("left") or mouse_button("right") then
    circle(mx, my, 10 * ts, pull > 0 and rgb(120, 230, 255) or rgb(255, 140, 40))
  end
  flip()
end
