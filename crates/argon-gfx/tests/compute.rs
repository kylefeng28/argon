//! Compute shaders, storage buffers and draw_points (needs any GPU adapter).

use argon_core::draw::{BufferId, DrawCmd, PointsOpts, ShaderId, ShaderInputs};
use argon_core::shader::{compile, compile_compute};
use argon_core::Color;
use argon_gfx::{Gpu, Renderer};
use std::sync::Arc;

fn renderer(w: u32, h: u32) -> Option<Renderer> {
    match Gpu::new(None) {
        Ok(gpu) => Some(Renderer::new(Arc::new(gpu), w, h)),
        Err(e) => {
            eprintln!("skipping: {e}");
            None
        }
    }
}

fn px(buf: &[u8], w: u32, x: u32, y: u32) -> Color {
    let i = ((y * w + x) * 4) as usize;
    Color::rgba(buf[i], buf[i + 1], buf[i + 2], buf[i + 3])
}

fn floats(bytes: &[u8]) -> Vec<f32> {
    bytes.as_chunks::<4>().0.iter().map(|c| f32::from_le_bytes(*c)).collect()
}

fn with_buffer(b0: BufferId) -> ShaderInputs {
    let mut i = ShaderInputs::default();
    i.buffers[0] = Some(b0);
    i
}

#[test]
fn dispatch_writes_buffer_and_sees_uniforms_and_icount() {
    let Some(mut r) = renderer(16, 16) else { return };
    let sh = compile_compute(
        r#"
layout(local_size_x = 64) in;
uniform float scale;
layout(std430, binding = 0) buffer Out { float v[]; };
void main() {
    uint i = gl_GlobalInvocationID.x;
    if (i >= iCount.x) return;
    v[i] = float(i) * scale;
}"#,
    )
    .unwrap();
    let mut inputs = with_buffer(BufferId(1));
    inputs.user[0] = [2.0, 0.0, 0.0, 0.0];
    r.render(
        &[
            DrawCmd::CreateBuffer { id: BufferId(1), bytes: 200 * 4 },
            DrawCmd::CreateShader { id: ShaderId(1), shader: sh },
            // 100 invocations of a 200-element buffer: the rest stays zero.
            DrawCmd::Dispatch { id: ShaderId(1), count: [100, 1, 1], inputs },
        ],
        None,
        false,
    );
    let v = floats(&r.read_buffer(BufferId(1)).unwrap());
    assert_eq!(v[0], 0.0);
    assert_eq!(v[10], 20.0);
    assert_eq!(v[99], 198.0);
    assert_eq!(v[100], 0.0, "iCount bounds the work");
}

#[test]
fn write_buffer_then_two_dispatches_ping_pong_in_order() {
    let Some(mut r) = renderer(16, 16) else { return };
    let add_one = compile_compute(
        r#"
layout(local_size_x = 4) in;
layout(std430, binding = 0) readonly buffer Src { float a[]; };
layout(std430, binding = 1) buffer Dst { float b[]; };
void main() { uint i = gl_GlobalInvocationID.x; b[i] = a[i] + 1.0; }"#,
    )
    .unwrap();
    let init: Vec<u8> = [10.0f32, 20.0, 30.0, 40.0].iter().flat_map(|f| f.to_le_bytes()).collect();
    let ab = {
        let mut i = ShaderInputs::default();
        i.buffers[0] = Some(BufferId(1));
        i.buffers[1] = Some(BufferId(2));
        i
    };
    let ba = {
        let mut i = ShaderInputs::default();
        i.buffers[0] = Some(BufferId(2));
        i.buffers[1] = Some(BufferId(1));
        i
    };
    r.render(
        &[
            DrawCmd::CreateBuffer { id: BufferId(1), bytes: 16 },
            DrawCmd::CreateBuffer { id: BufferId(2), bytes: 16 },
            DrawCmd::WriteBuffer { id: BufferId(1), offset: 0, data: init },
            DrawCmd::CreateShader { id: ShaderId(1), shader: add_one },
            DrawCmd::Dispatch { id: ShaderId(1), count: [4, 1, 1], inputs: ab },
            DrawCmd::Dispatch { id: ShaderId(1), count: [4, 1, 1], inputs: ba },
        ],
        None,
        false,
    );
    assert_eq!(floats(&r.read_buffer(BufferId(1)).unwrap()), vec![12.0, 22.0, 32.0, 42.0]);
}

#[test]
fn draw_points_from_a_computed_buffer_in_the_same_frame() {
    let Some(mut r) = renderer(32, 32) else { return };
    // Put 3 points (vec4: xy = position) on a diagonal.
    let place = compile_compute(
        r#"
layout(local_size_x = 1) in;
layout(std430, binding = 0) buffer P { vec4 p[]; };
void main() { uint i = gl_GlobalInvocationID.x; p[i] = vec4(4.0 + float(i) * 10.0, 4.0 + float(i) * 10.0, 0.0, 0.0); }"#,
    )
    .unwrap();
    let red = Color::rgb(255, 0, 0);
    r.render(
        &[
            DrawCmd::Clear(Color::BLACK),
            DrawCmd::CreateBuffer { id: BufferId(1), bytes: 3 * 16 },
            DrawCmd::CreateShader { id: ShaderId(1), shader: place },
            DrawCmd::Dispatch { id: ShaderId(1), count: [3, 1, 1], inputs: with_buffer(BufferId(1)) },
            DrawCmd::DrawPoints { buffer: BufferId(1), opts: PointsOpts { count: 3, stride: 4, colors: None, color: red, size: 3.0 } },
        ],
        None,
        false,
    );
    let fb = r.read_framebuffer();
    for c in [4, 14, 24] {
        assert_eq!(px(&fb, 32, c, c), red, "point at {c}");
        assert_eq!(px(&fb, 32, c - 1, c + 1), red, "3x3 square at {c}");
    }
    assert_eq!(px(&fb, 32, 9, 9), Color::BLACK);
    assert_eq!(px(&fb, 32, 0, 31), Color::BLACK);
}

#[test]
fn per_point_colors_and_fragment_shader_reading_a_buffer() {
    let Some(mut r) = renderer(8, 4) else { return };
    // Buffer of 8 floats: brightness per column, read by a fragment shader.
    let vals: Vec<u8> = (0..8).map(|i| i as f32 / 7.0).flat_map(|f| f.to_le_bytes()).collect();
    let show = compile(
        r#"
layout(std430, binding = 0) buffer V { float v[]; };
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    fragColor = vec4(v[uint(fragCoord.x)], 0.0, 0.0, 1.0);
}"#,
    )
    .unwrap();
    // One green point with a per-point color buffer at (6, 1).
    let pos: Vec<u8> = [6.0f32, 1.0].iter().flat_map(|f| f.to_le_bytes()).collect();
    let col: Vec<u8> = [0.0f32, 1.0, 0.0, 1.0].iter().flat_map(|f| f.to_le_bytes()).collect();
    r.render(
        &[
            DrawCmd::CreateBuffer { id: BufferId(1), bytes: 32 },
            DrawCmd::WriteBuffer { id: BufferId(1), offset: 0, data: vals },
            DrawCmd::CreateShader { id: ShaderId(1), shader: show },
            DrawCmd::DrawShader { id: ShaderId(1), rect: None, inputs: with_buffer(BufferId(1)) },
            DrawCmd::CreateBuffer { id: BufferId(2), bytes: 8 },
            DrawCmd::WriteBuffer { id: BufferId(2), offset: 0, data: pos },
            DrawCmd::CreateBuffer { id: BufferId(3), bytes: 16 },
            DrawCmd::WriteBuffer { id: BufferId(3), offset: 0, data: col },
            DrawCmd::DrawPoints {
                buffer: BufferId(2),
                opts: PointsOpts { count: 1, stride: 2, colors: Some(BufferId(3)), color: Color::WHITE, size: 1.0 },
            },
        ],
        None,
        false,
    );
    let fb = r.read_framebuffer();
    assert_eq!(px(&fb, 8, 0, 2).r, 0);
    assert_eq!(px(&fb, 8, 7, 2).r, 255);
    assert!((70..=76).contains(&px(&fb, 8, 2, 2).r), "{:?}", px(&fb, 8, 2, 2));
    assert_eq!(px(&fb, 8, 6, 1), Color::rgb(0, 255, 0));
}
