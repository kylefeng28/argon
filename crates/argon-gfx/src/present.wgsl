// Presenter: blits the composite onto the window with optional display
// filters. Bits: 1 glow, 2 scanlines, 4 vignette, 8 glass.
//
// "Glass" is the Tron: Legacy light-ribbon look: every bright stroke reads
// as a lit glass tube: a white-hot core, a colored halo with a little
// chromatic dispersion, and a specular rim where light catches the edge.

struct Params {
    // 1 / framebuffer size (filter radii are in framebuffer pixels)
    texel: vec2<f32>,
    mode: u32,
    // framebuffer rows (for scanlines)
    rows: f32,
    // destination rect in window pixels: x, y, w, h
    rect: vec4<f32>,
};

struct VsIn {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;
    out.pos = vec4<f32>(in.pos, 0.0, 1.0);
    out.uv = in.uv;
    return out;
}

@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
@group(0) @binding(2) var<uniform> params: Params;

fn lum(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.299, 0.587, 0.114));
}

fn tap(uv: vec2<f32>, dx: f32, dy: f32, r: f32) -> vec3<f32> {
    return textureSample(tex, samp, uv + params.texel * vec2<f32>(dx, dy) * r).rgb;
}

// 13-tap disc blur at radius r (framebuffer pixels): center, a ring at r/2
// and a ring at r. Round, so halos do not look boxy.
fn disc(uv: vec2<f32>, r: f32) -> vec3<f32> {
    var acc = textureSample(tex, samp, uv).rgb * 0.2;
    acc += (tap(uv, 0.5, 0.0, r) + tap(uv, -0.5, 0.0, r) + tap(uv, 0.0, 0.5, r) + tap(uv, 0.0, -0.5, r)) * 0.1;
    // acc += (tap(uv, 1.0, 0.0, r) + tap(uv, -1.0, 0.0, r) + tap(uv, 0.0, 1.0, r) + tap(uv, 0.0, -1.0, r)) * 0.05;
    acc += (tap(uv, 0.707, 0.707, r) + tap(uv, -0.707, 0.707, r) + tap(uv, 0.707, -0.707, r) + tap(uv, -0.707, -0.707, r)) * 0.05;
    return acc;
}

// Soft light field: how much light surrounds this point.
fn field(uv: vec2<f32>) -> vec3<f32> {
    return disc(uv, 1.5) * 0.25 + disc(uv, 3.0) * 0.25 + disc(uv, 6.0) * 0.25 + disc(uv, 10.0) * 0.25;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let base = textureSample(tex, samp, uv).rgb;
    var color = base;
    let base_lum = lum(base);
    let glow_on = (params.mode & 1u) != 0u;
    let glass_on = (params.mode & 8u) != 0u;

    if (glow_on || glass_on) {
        let f = field(uv);
        if (glow_on) {
            // Halo: the field minus what is already lit here, so strokes get
            // a surrounding glow and fills are not washed out. Slight
            // dispersion: red spreads a touch wider than blue.
            var halo = max(f - base, vec3<f32>(0.0));
            if (glass_on) {
                let fr = disc(uv, 11.0);
                halo.r = max(halo.r, max(fr.r - base.r, 0.0) * 0.6);
            }
            color += halo * 2.0;
            // Lit strokes themselves get a little hotter (keeps text punchy),
            // and the very brightest go white at the core.
            color += base * 0.35 * smoothstep(0.25, 0.7, base_lum);
            let core = pow(clamp(base_lum, 0.0, 1.0), 6.0);
            color = mix(color, vec3<f32>(1.0), core * 0.3);
        }
        if (glass_on) {
            // Specular rim: light from the upper left catches the edge of
            // every lit stroke; the opposite edge falls into a soft shadow.
            let d = params.texel * 1.5;
            let gx = lum(disc(uv + vec2<f32>(d.x, 0.0), 2.0)) - lum(disc(uv - vec2<f32>(d.x, 0.0), 2.0));
            let gy = lum(disc(uv + vec2<f32>(0.0, d.y), 2.0)) - lum(disc(uv - vec2<f32>(0.0, d.y), 2.0));
            let g = vec2<f32>(gx, gy);
            let light = vec2<f32>(-0.6, -0.8);
            let facing = dot(g, light);
            let rim = clamp(facing * 4.0, 0.0, 1.0) * (1.0 - smoothstep(0.5, 0.9, base_lum));
            color += vec3<f32>(0.85, 0.97, 1.0) * rim * 0.45;
            let shade = clamp(-facing * 3.0, 0.0, 1.0) * (1.0 - smoothstep(0.3, 0.7, base_lum));
            color = color * (1.0 - shade * 0.35);
            // Liquid sheen: a faint, slowly varying tint over lit areas.
            let sheen = (sin(uv.x * 40.0 + uv.y * 25.0) * 0.5 + 0.5) * 0.06;
            color += vec3<f32>(0.6, 0.9, 1.0) * sheen * smoothstep(0.05, 0.3, lum(f));
        }
    }
    if ((params.mode & 2u) != 0u) {
        // One dark line per framebuffer row.
        let fy = (in.pos.y - params.rect.y) / params.rect.w * params.rows;
        let phase = fract(fy);
        let line = 0.8 + 0.2 * smoothstep(0.0, 0.5, abs(phase - 0.5) * 2.0);
        color = color * line;
    }
    if ((params.mode & 4u) != 0u) {
        let dv = uv - vec2<f32>(0.5, 0.5);
        let v = 1.0 - dot(dv, dv) * 0.8;
        color = color * clamp(v, 0.0, 1.0);
    }
    return vec4<f32>(color, 1.0);
}
