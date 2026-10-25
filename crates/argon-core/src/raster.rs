//! Deterministic CPU rasterization of primitives into horizontal pixel spans.
//! The GPU only ever draws axis-aligned pixel rectangles for Tier 1
//! primitives, so output is identical on every GPU (NFR-8).

/// A horizontal run of pixels on row `y` covering `x0..=x1` inclusive.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Span {
    pub y: i32,
    pub x0: i32,
    pub x1: i32,
}

impl Span {
    pub fn new(y: i32, x0: i32, x1: i32) -> Span {
        if x0 <= x1 {
            Span { y, x0, x1 }
        } else {
            Span { y, x0: x1, x1: x0 }
        }
    }
}

/// Bresenham line as 1-pixel spans, merging horizontal runs.
pub fn line(x1: i32, y1: i32, x2: i32, y2: i32, out: &mut Vec<Span>) {
    let dx = (x2 - x1).abs();
    let dy = -(y2 - y1).abs();
    let sx = if x1 < x2 { 1 } else { -1 };
    let sy = if y1 < y2 { 1 } else { -1 };
    let mut err = dx + dy;
    let (mut x, mut y) = (x1, y1);
    let mut run_start = x;
    loop {
        let at_end = x == x2 && y == y2;
        let e2 = 2 * err;
        let mut step_y = false;
        let mut nx = x;
        if e2 >= dy {
            err += dy;
            nx = x + sx;
        }
        if e2 <= dx {
            err += dx;
            step_y = true;
        }
        if at_end || step_y {
            out.push(Span::new(y, run_start, x));
            if at_end {
                break;
            }
            y += sy;
            x = nx;
            run_start = x;
        } else {
            x = nx;
        }
    }
}

/// Filled axis-aligned rectangle (w, h may be negative; empty if zero).
pub fn fill_rect(x: i32, y: i32, w: i32, h: i32, out: &mut Vec<Span>) {
    if w == 0 || h == 0 {
        return;
    }
    let (x0, x1) = if w > 0 { (x, x + w - 1) } else { (x + w + 1, x) };
    let (y0, y1) = if h > 0 { (y, y + h - 1) } else { (y + h + 1, y) };
    for yy in y0..=y1 {
        out.push(Span::new(yy, x0, x1));
    }
}

/// 1-pixel rectangle outline.
pub fn rect(x: i32, y: i32, w: i32, h: i32, out: &mut Vec<Span>) {
    if w == 0 || h == 0 {
        return;
    }
    let (x0, x1) = if w > 0 { (x, x + w - 1) } else { (x + w + 1, x) };
    let (y0, y1) = if h > 0 { (y, y + h - 1) } else { (y + h + 1, y) };
    out.push(Span::new(y0, x0, x1));
    if y1 != y0 {
        out.push(Span::new(y1, x0, x1));
    }
    for yy in (y0 + 1)..y1 {
        out.push(Span::new(yy, x0, x0));
        if x1 != x0 {
            out.push(Span::new(yy, x1, x1));
        }
    }
}

/// Ellipse as spans. `fill` emits one span per row; otherwise the outline
/// pixels. A circle is `ellipse(x, y, r, r)`.
pub fn ellipse(cx: i32, cy: i32, rx: i32, ry: i32, fill: bool, out: &mut Vec<Span>) {
    let rx = rx.abs();
    let ry = ry.abs();
    if rx == 0 && ry == 0 {
        out.push(Span::new(cy, cx, cx));
        return;
    }
    if ry == 0 {
        out.push(Span::new(cy, cx - rx, cx + rx));
        return;
    }
    if rx == 0 {
        for y in -ry..=ry {
            out.push(Span::new(cy + y, cx, cx));
        }
        return;
    }
    // Per-row x extent: ex(dy) = rx * sqrt(1 - (dy/ry)^2), rounded. f64 sqrt
    // is correctly rounded on every platform, so this is deterministic.
    let xs: Vec<i32> = (0..=ry)
        .map(|dy| {
            let t = 1.0 - (dy as f64 / ry as f64).powi(2);
            (rx as f64 * t.max(0.0).sqrt()).round() as i32
        })
        .collect();

    if fill {
        for (dy, &ex) in xs.iter().enumerate() {
            let dy = dy as i32;
            out.push(Span::new(cy - dy, cx - ex, cx + ex));
            if dy != 0 {
                out.push(Span::new(cy + dy, cx - ex, cx + ex));
            }
        }
    } else {
        for dy in 0..=ry {
            let ex = xs[dy as usize];
            // Outline: pixels between the extent of this row and the next
            // (closer to the pole) row, so the curve stays connected.
            let inner = if dy == ry { 0 } else { xs[dy as usize + 1] + 1 };
            let inner = inner.min(ex);
            for &row in &[cy - dy, cy + dy] {
                if inner == 0 {
                    out.push(Span::new(row, cx - ex, cx + ex));
                } else {
                    out.push(Span::new(row, cx - ex, cx - inner));
                    out.push(Span::new(row, cx + inner, cx + ex));
                }
                if dy == 0 {
                    break;
                }
            }
        }
    }
}

/// Scanline fill of a simple polygon (even-odd rule), pixel centers sampled.
pub fn fill_polygon(pts: &[[f32; 2]], out: &mut Vec<Span>) {
    if pts.len() < 3 {
        return;
    }
    let min_y = pts.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min).floor() as i32;
    let max_y = pts.iter().map(|p| p[1]).fold(f32::NEG_INFINITY, f32::max).ceil() as i32;
    let mut xs: Vec<f32> = Vec::with_capacity(pts.len());
    for y in min_y..max_y {
        let sy = y as f32 + 0.5;
        xs.clear();
        for i in 0..pts.len() {
            let a = pts[i];
            let b = pts[(i + 1) % pts.len()];
            if (a[1] <= sy && b[1] > sy) || (b[1] <= sy && a[1] > sy) {
                let t = (sy - a[1]) / (b[1] - a[1]);
                xs.push(a[0] + t * (b[0] - a[0]));
            }
        }
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        for pair in xs.chunks(2) {
            if pair.len() == 2 {
                let x0 = (pair[0] - 0.5).ceil() as i32;
                let x1 = (pair[1] - 0.5).floor() as i32;
                if x1 >= x0 {
                    out.push(Span::new(y, x0, x1));
                }
            }
        }
    }
}

/// Polygon outline as lines between consecutive points (closed).
pub fn polygon_outline(pts: &[[f32; 2]], out: &mut Vec<Span>) {
    for i in 0..pts.len() {
        let a = pts[i];
        let b = pts[(i + 1) % pts.len()];
        line(a[0].round() as i32, a[1].round() as i32, b[0].round() as i32, b[1].round() as i32, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn pixels(spans: &[Span]) -> HashSet<(i32, i32)> {
        spans.iter().flat_map(|s| (s.x0..=s.x1).map(move |x| (x, s.y))).collect()
    }

    #[test]
    fn line_endpoints_and_length() {
        let mut v = Vec::new();
        line(0, 0, 10, 3, &mut v);
        let p = pixels(&v);
        assert!(p.contains(&(0, 0)) && p.contains(&(10, 3)));
        assert_eq!(p.len(), 11);
        v.clear();
        line(5, 5, 5, 5, &mut v);
        assert_eq!(pixels(&v).len(), 1);
        v.clear();
        line(0, 0, 0, 7, &mut v);
        assert_eq!(pixels(&v).len(), 8);
    }

    #[test]
    fn rect_outline_count() {
        let mut v = Vec::new();
        rect(0, 0, 4, 3, &mut v);
        assert_eq!(pixels(&v).len(), 4 * 3 - 2);
        v.clear();
        fill_rect(0, 0, 4, 3, &mut v);
        assert_eq!(pixels(&v).len(), 12);
    }

    #[test]
    fn circle_is_symmetric_and_connected() {
        let mut v = Vec::new();
        ellipse(20, 20, 7, 7, true, &mut v);
        let p = pixels(&v);
        assert!(p.contains(&(27, 20)) && p.contains(&(13, 20)) && p.contains(&(20, 27)) && p.contains(&(20, 13)));
        for &(x, y) in &p {
            assert!(p.contains(&(40 - x, y)) && p.contains(&(x, 40 - y)));
        }
        // Area roughly pi r^2
        assert!((p.len() as f32 - std::f32::consts::PI * 49.0).abs() < 20.0);
        v.clear();
        ellipse(20, 20, 7, 7, false, &mut v);
        let o = pixels(&v);
        assert!(o.len() < p.len() && o.len() > 30);
    }

    #[test]
    fn polygon_fill_square() {
        let mut v = Vec::new();
        fill_polygon(&[[0.0, 0.0], [4.0, 0.0], [4.0, 4.0], [0.0, 4.0]], &mut v);
        assert_eq!(pixels(&v).len(), 16);
    }
}
