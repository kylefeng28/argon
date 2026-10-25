//! Headless rendering tests. They need a GPU adapter (a software one such as
//! SwiftShader/lavapipe is fine) and are skipped when none is available.

use argon_core::draw::{DrawCmd, ImageId, Rect, SpriteOpts};
use argon_core::{Cell, Color, TextGrid};
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
fn primitives_land_on_exact_pixels() {
    let Some(mut r) = renderer(64, 32) else { return };
    let red = Color::rgb(255, 0, 0);
    let green = Color::rgb(0, 255, 0);
    let blue = Color::rgb(0, 0, 255);
    r.render(
        &[
            DrawCmd::Clear(Color::BLACK),
            DrawCmd::Pixel { x: 3, y: 5, color: red },
            DrawCmd::Rect { x: 10, y: 10, w: 4, h: 3, color: green, fill: true },
            DrawCmd::Line { x1: 20, y1: 0, x2: 20, y2: 31, color: blue },
        ],
        None,
        false,
    );
    let buf = r.read_composite();
    assert_eq!(px(&buf, 64, 3, 5), red);
    assert_eq!(px(&buf, 64, 4, 5), Color::BLACK);
    assert_eq!(px(&buf, 64, 2, 5), Color::BLACK);
    assert_eq!(px(&buf, 64, 10, 10), green);
    assert_eq!(px(&buf, 64, 13, 12), green);
    assert_eq!(px(&buf, 64, 14, 12), Color::BLACK);
    assert_eq!(px(&buf, 64, 9, 10), Color::BLACK);
    assert_eq!(px(&buf, 64, 20, 0), blue);
    assert_eq!(px(&buf, 64, 20, 31), blue);
    assert_eq!(px(&buf, 64, 21, 15), Color::BLACK);
}

#[test]
fn framebuffer_persists_between_frames() {
    let Some(mut r) = renderer(16, 16) else { return };
    let red = Color::rgb(255, 0, 0);
    r.render(&[DrawCmd::Clear(Color::BLACK), DrawCmd::Pixel { x: 1, y: 1, color: red }], None, false);
    r.render(&[], None, false);
    let buf = r.read_composite();
    assert_eq!(px(&buf, 16, 1, 1), red);
}

#[test]
fn text_layer_draws_glyphs_over_framebuffer() {
    let Some(mut r) = renderer(64, 32) else { return };
    let mut grid = TextGrid::new(8, 2);
    grid.put(0, 0, Cell { ch: '\u{2588}', fg: Color::WHITE, bg: None }); // full block
    grid.put(1, 0, Cell { ch: ' ', fg: Color::WHITE, bg: Some(Color::rgb(0, 0, 255)) });
    r.render(&[DrawCmd::Clear(Color::rgb(40, 40, 40))], Some(&grid), false);
    let buf = r.read_composite();
    // Full block glyph covers its whole cell.
    assert_eq!(px(&buf, 64, 0, 0), Color::WHITE);
    assert_eq!(px(&buf, 64, 7, 15), Color::WHITE);
    // Cell background.
    assert_eq!(px(&buf, 64, 8, 0), Color::rgb(0, 0, 255));
    // Transparent cell shows the framebuffer.
    assert_eq!(px(&buf, 64, 20, 5), Color::rgb(40, 40, 40));
    // Text is not baked into the framebuffer.
    let fb = r.read_framebuffer();
    assert_eq!(px(&fb, 64, 0, 0), Color::rgb(40, 40, 40));
}

#[test]
fn sprites_and_render_targets() {
    let Some(mut r) = renderer(32, 32) else { return };
    let id = ImageId(1);
    // 2x2 image: red, green / blue, white
    let pixels = vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255];
    r.render(
        &[
            DrawCmd::Clear(Color::BLACK),
            DrawCmd::CreateImage { id, width: 2, height: 2, pixels },
            DrawCmd::Sprite {
                image: id,
                src: Rect::new(0.0, 0.0, 2.0, 2.0),
                x: 10.0,
                y: 10.0,
                opts: SpriteOpts { scale_x: 4.0, scale_y: 4.0, ..Default::default() },
            },
            // Draw into the image, then draw it again elsewhere.
            DrawCmd::SetTarget(Some(id)),
            DrawCmd::Pixel { x: 0, y: 0, color: Color::rgb(0, 255, 255) },
            DrawCmd::SetTarget(None),
            DrawCmd::Sprite { image: id, src: Rect::new(0.0, 0.0, 2.0, 2.0), x: 0.0, y: 0.0, opts: SpriteOpts::default() },
        ],
        None,
        false,
    );
    let buf = r.read_composite();
    assert_eq!(px(&buf, 32, 10, 10), Color::rgb(255, 0, 0));
    assert_eq!(px(&buf, 32, 14, 10), Color::rgb(0, 255, 0));
    assert_eq!(px(&buf, 32, 10, 14), Color::rgb(0, 0, 255));
    assert_eq!(px(&buf, 32, 17, 17), Color::WHITE);
    assert_eq!(px(&buf, 32, 0, 0), Color::rgb(0, 255, 255));
    assert_eq!(px(&buf, 32, 1, 0), Color::rgb(0, 255, 0));
}
