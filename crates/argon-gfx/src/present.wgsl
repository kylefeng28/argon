// Presenter: blits the composite onto the window with optional display
// filters: glow (bit 1), scanlines (bit 2), vignette (bit 4).

struct Params {
    // 1 / composite size
    texel: vec2<f32>,
    // display filter bits
    mode: u32,
    // integer scale factor composite -> window
    scale: f32,
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

fn blur(uv: vec2<f32>, r: f32) -> vec3<f32> {
    let t = params.texel * r;
    var acc = vec3<f32>(0.0);
    acc += textureSample(tex, samp, uv + vec2<f32>(-t.x, -t.y)).rgb * 0.0625;
    acc += textureSample(tex, samp, uv + vec2<f32>(0.0, -t.y)).rgb * 0.125;
    acc += textureSample(tex, samp, uv + vec2<f32>(t.x, -t.y)).rgb * 0.0625;
    acc += textureSample(tex, samp, uv + vec2<f32>(-t.x, 0.0)).rgb * 0.125;
    acc += textureSample(tex, samp, uv).rgb * 0.25;
    acc += textureSample(tex, samp, uv + vec2<f32>(t.x, 0.0)).rgb * 0.125;
    acc += textureSample(tex, samp, uv + vec2<f32>(-t.x, t.y)).rgb * 0.0625;
    acc += textureSample(tex, samp, uv + vec2<f32>(0.0, t.y)).rgb * 0.125;
    acc += textureSample(tex, samp, uv + vec2<f32>(t.x, t.y)).rgb * 0.0625;
    return acc;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let base = textureSample(tex, samp, in.uv).rgb;
    var color = base;

    if ((params.mode & 1u) != 0u) {
        // Bloom: wide soft halo around bright pixels, kept subtle on dark
        // backgrounds so black stays black.
        let b1 = blur(in.uv, 1.0);
        let b2 = blur(in.uv, 2.5);
        let b3 = blur(in.uv, 5.0);
        let halo = (b1 * 0.35 + b2 * 0.35 + b3 * 0.3);
        color = base + halo * 0.55;
    }
    if ((params.mode & 2u) != 0u) {
        // Scanlines in window pixels: one dark line per composite row.
        let y = (in.pos.y - params.rect.y) / params.scale;
        let phase = fract(y);
        let line = 0.78 + 0.22 * smoothstep(0.0, 0.5, abs(phase - 0.5) * 2.0);
        color = color * line;
    }
    if ((params.mode & 4u) != 0u) {
        let d = in.uv - vec2<f32>(0.5, 0.5);
        let v = 1.0 - dot(d, d) * 0.9;
        color = color * clamp(v, 0.0, 1.0);
    }
    return vec4<f32>(color, 1.0);
}
