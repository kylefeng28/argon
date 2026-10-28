//! User shader rendering through the real pipeline (needs any GPU adapter).

use argon_core::draw::{DrawCmd, ImageId, Rect, ShaderId, ShaderInputs, SpriteOpts};
use argon_core::shader::compile;
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

#[test]
fn full_screen_shader_uses_fragcoord_and_uniforms() {
    let Some(mut r) = renderer(64, 32) else { return };
    // Left half red, right half uses the user uniform as green; y-up check: top row blue.
    let sh = compile(
        r#"
uniform float g;
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    if (fragCoord.y > iResolution.y - 1.0) { fragColor = vec4(0.0, 0.0, 1.0, 1.0); return; }
    fragColor = fragCoord.x < iResolution.x * 0.5 ? vec4(1.0, 0.0, 0.0, 1.0) : vec4(0.0, g, 0.0, 1.0);
}"#,
    )
    .unwrap();
    let mut inputs = ShaderInputs::default();
    inputs.user[0] = [0.5, 0.0, 0.0, 0.0];
    r.render(
        &[
            DrawCmd::Clear(Color::BLACK),
            DrawCmd::CreateShader { id: ShaderId(1), shader: sh },
            DrawCmd::DrawShader { id: ShaderId(1), rect: None, inputs },
        ],
        None,
        false,
    );
    let buf = r.read_composite();
    assert_eq!(px(&buf, 64, 5, 16), Color::rgb(255, 0, 0));
    let g = px(&buf, 64, 60, 16);
    assert!(g.r == 0 && (126..=129).contains(&g.g) && g.b == 0, "{g:?}");
    // Shadertoy origin is bottom-left: fragCoord.y max is the top row of the image.
    assert_eq!(px(&buf, 64, 10, 0), Color::rgb(0, 0, 255));
    assert_eq!(px(&buf, 64, 10, 31), Color::rgb(255, 0, 0));
}

#[test]
fn rect_shader_and_channel_sampling() {
    let Some(mut r) = renderer(32, 32) else { return };
    let id = ImageId(1);
    // 2x2 image: top row red | green, bottom row blue | white
    let pixels = vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255];
    let sh = compile(
        r#"
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;
    fragColor = texture(iChannel0, uv);
}"#,
    )
    .unwrap();
    let inputs = ShaderInputs { channels: [Some(id), None, None, None], ..Default::default() };
    r.render(
        &[
            DrawCmd::Clear(Color::BLACK),
            DrawCmd::CreateImage { id, width: 2, height: 2, pixels },
            DrawCmd::CreateShader { id: ShaderId(1), shader: sh },
            DrawCmd::DrawShader { id: ShaderId(1), rect: Some(Rect::new(8.0, 8.0, 16.0, 16.0)), inputs },
        ],
        None,
        false,
    );
    let buf = r.read_composite();
    // The image appears upright: red/green on top, blue/white below.
    assert_eq!(px(&buf, 32, 10, 10), Color::rgb(255, 0, 0));
    assert_eq!(px(&buf, 32, 21, 10), Color::rgb(0, 255, 0));
    assert_eq!(px(&buf, 32, 10, 21), Color::rgb(0, 0, 255));
    assert_eq!(px(&buf, 32, 21, 21), Color::WHITE);
    assert_eq!(px(&buf, 32, 2, 2), Color::BLACK, "outside the rect untouched");
}

#[test]
fn set_shader_applies_to_sprites_with_sheet_as_channel0() {
    let Some(mut r) = renderer(32, 32) else { return };
    let id = ImageId(1);
    let pixels = vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255];
    // Swap red and blue of the sprite texture.
    let sh = compile(
        r#"
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec4 c = textureTopDown(iChannel0, vUv) * vColor;
    fragColor = vec4(c.b, c.g, c.r, c.a);
}"#,
    )
    .unwrap();
    r.render(
        &[
            DrawCmd::Clear(Color::BLACK),
            DrawCmd::CreateImage { id, width: 2, height: 2, pixels },
            DrawCmd::CreateShader { id: ShaderId(1), shader: sh },
            DrawCmd::SetShader { id: Some(ShaderId(1)), inputs: ShaderInputs::default() },
            DrawCmd::Sprite {
                image: id,
                src: Rect::new(0.0, 0.0, 2.0, 2.0),
                x: 0.0,
                y: 0.0,
                opts: SpriteOpts { scale_x: 8.0, scale_y: 8.0, ..Default::default() },
            },
            DrawCmd::SetShader { id: None, inputs: ShaderInputs::default() },
            DrawCmd::Sprite { image: id, src: Rect::new(0.0, 0.0, 2.0, 2.0), x: 20.0, y: 20.0, opts: SpriteOpts::default() },
        ],
        None,
        false,
    );
    let buf = r.read_composite();
    assert_eq!(px(&buf, 32, 2, 2), Color::rgb(0, 0, 255), "red became blue through the shader");
    assert_eq!(px(&buf, 32, 10, 2), Color::rgb(0, 255, 0));
    assert_eq!(px(&buf, 32, 20, 20), Color::rgb(255, 0, 0), "unshaded sprite keeps red");
}
