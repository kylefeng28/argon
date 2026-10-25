// draw_points: one pixel-space square per buffer element, read straight
// from a storage buffer (no vertex buffer, no CPU round trip).

struct Params {
    // render target size in pixels
    target_size: vec2<f32>,
    // camera offset in pixels
    camera: vec2<f32>,
    // multiplied with per-point colors (or used alone)
    color: vec4<f32>,
    // square size in pixels
    size: f32,
    // floats per element in `pos` (2 for vec2, 4 for vec4)
    stride: u32,
    // 1 when `cols` holds a vec4 color per point
    use_colors: u32,
    _pad: u32,
};

@group(0) @binding(0) var<uniform> P: Params;
@group(0) @binding(1) var<storage, read> pos: array<f32>;
@group(0) @binding(2) var<storage, read> cols: array<f32>;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> VsOut {
    // Two triangles: (0,0) (1,0) (1,1) / (0,0) (1,1) (0,1)
    let cx = select(0.0, 1.0, vi == 1u || vi == 2u || vi == 4u);
    let cy = select(0.0, 1.0, vi == 2u || vi == 4u || vi == 5u);
    let b = ii * P.stride;
    let xy = vec2<f32>(pos[b], pos[b + 1u]) + P.camera;
    // Center the square on the point, snapped to whole pixels.
    let origin = floor(xy - vec2<f32>(P.size * 0.5) + vec2<f32>(0.5));
    let q = origin + vec2<f32>(cx, cy) * P.size;
    var out: VsOut;
    out.pos = vec4<f32>(q.x / P.target_size.x * 2.0 - 1.0, 1.0 - q.y / P.target_size.y * 2.0, 0.0, 1.0);
    var c = P.color;
    if (P.use_colors != 0u) {
        let k = ii * 4u;
        c = c * vec4<f32>(cols[k], cols[k + 1u], cols[k + 2u], cols[k + 3u]);
    }
    out.color = c;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return in.color;
}
