//! The text layer: a fixed grid of character cells with a cursor. Used by
//! the shell, the editor and by programs through `print`/`locate`.

use crate::color::Color;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Cell {
    pub ch: char,
    pub fg: Color,
    /// `None` means transparent: the pixel framebuffer shows through.
    pub bg: Option<Color>,
}

impl Cell {
    pub const fn blank() -> Cell {
        Cell { ch: ' ', fg: Color::WHITE, bg: None }
    }

    pub fn is_blank(&self) -> bool {
        self.ch == ' ' && self.bg.is_none()
    }
}

#[derive(Clone, Debug)]
pub struct TextGrid {
    cols: usize,
    rows: usize,
    /// Cell size multiplier over the 8x16 base (see `text_scale_for_height`).
    cell_scale: u32,
    cells: Vec<Cell>,
    pub cursor_col: usize,
    pub cursor_row: usize,
    pub cursor_visible: bool,
    pub fg: Color,
    pub bg: Option<Color>,
    /// Bumped on every mutation so the renderer can skip re-uploads.
    pub generation: u64,
}

impl TextGrid {
    pub fn new(cols: usize, rows: usize) -> TextGrid {
        TextGrid {
            cols,
            rows,
            cell_scale: 1,
            cells: vec![Cell::blank(); cols * rows],
            cursor_col: 0,
            cursor_row: 0,
            cursor_visible: false,
            fg: Color::WHITE,
            bg: None,
            generation: 0,
        }
    }

    /// Grid that covers a pixel area of the given size, with cells scaled so
    /// the grid stays near 80x25 at any resolution.
    pub fn for_pixels(width: u32, height: u32) -> TextGrid {
        let s = crate::text_scale_for_height(height);
        let mut g = TextGrid::new(
            (width / (crate::CELL_W * s)).max(1) as usize,
            (height / (crate::CELL_H * s)).max(1) as usize,
        );
        g.cell_scale = s;
        g
    }

    pub fn cell_scale(&self) -> u32 {
        self.cell_scale
    }

    /// Cell size in pixels.
    pub fn cell_size(&self) -> (u32, u32) {
        (crate::CELL_W * self.cell_scale, crate::CELL_H * self.cell_scale)
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    pub fn get(&self, col: usize, row: usize) -> Option<&Cell> {
        if col < self.cols && row < self.rows {
            Some(&self.cells[row * self.cols + col])
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.cells.fill(Cell::blank());
        self.cursor_col = 0;
        self.cursor_row = 0;
        self.generation += 1;
    }

    /// Clear with the current background filling every cell.
    pub fn clear_with(&mut self, fg: Color, bg: Option<Color>) {
        self.fg = fg;
        self.bg = bg;
        self.cells.fill(Cell { ch: ' ', fg, bg });
        self.cursor_col = 0;
        self.cursor_row = 0;
        self.generation += 1;
    }

    pub fn put(&mut self, col: usize, row: usize, cell: Cell) {
        if col < self.cols && row < self.rows {
            self.cells[row * self.cols + col] = cell;
            self.generation += 1;
        }
    }

    pub fn put_char(&mut self, col: usize, row: usize, ch: char) {
        self.put(col, row, Cell { ch, fg: self.fg, bg: self.bg });
    }

    pub fn put_char_colored(&mut self, col: usize, row: usize, ch: char, fg: Color, bg: Option<Color>) {
        self.put(col, row, Cell { ch, fg, bg });
    }

    /// Write a string starting at (col, row) without moving the cursor or
    /// wrapping; clipped at the right edge.
    pub fn put_str(&mut self, col: usize, row: usize, s: &str) {
        for (i, ch) in s.chars().enumerate() {
            if col + i >= self.cols {
                break;
            }
            self.put_char(col + i, row, ch);
        }
    }

    pub fn put_str_colored(&mut self, col: usize, row: usize, s: &str, fg: Color, bg: Option<Color>) {
        for (i, ch) in s.chars().enumerate() {
            if col + i >= self.cols {
                break;
            }
            self.put_char_colored(col + i, row, ch, fg, bg);
        }
    }

    /// Fill a rectangular region of cells.
    pub fn fill_rect(&mut self, col: usize, row: usize, w: usize, h: usize, cell: Cell) {
        for r in row..(row + h).min(self.rows) {
            for c in col..(col + w).min(self.cols) {
                self.cells[r * self.cols + c] = cell;
            }
        }
        self.generation += 1;
    }

    pub fn locate(&mut self, col: usize, row: usize) {
        self.cursor_col = col.min(self.cols.saturating_sub(1));
        self.cursor_row = row.min(self.rows.saturating_sub(1));
    }

    pub fn scroll_up(&mut self, lines: usize) {
        let lines = lines.min(self.rows);
        self.cells.drain(..lines * self.cols);
        self.cells.extend(std::iter::repeat_n(Cell { ch: ' ', fg: self.fg, bg: self.bg }, lines * self.cols));
        self.generation += 1;
    }

    /// Terminal-style write at the cursor: wraps, handles '\n', '\r', '\t'
    /// and scrolls when passing the last row.
    pub fn write(&mut self, s: &str) {
        for ch in s.chars() {
            match ch {
                '\n' => self.newline(),
                '\r' => self.cursor_col = 0,
                '\t' => {
                    let next = (self.cursor_col / 8 + 1) * 8;
                    while self.cursor_col < next.min(self.cols) {
                        self.write_char(' ');
                    }
                }
                _ => self.write_char(ch),
            }
        }
    }

    fn write_char(&mut self, ch: char) {
        if self.cursor_col >= self.cols {
            self.newline();
        }
        let (c, r) = (self.cursor_col, self.cursor_row);
        self.put_char(c, r, ch);
        self.cursor_col += 1;
    }

    pub fn newline(&mut self) {
        self.cursor_col = 0;
        if self.cursor_row + 1 >= self.rows {
            self.scroll_up(1);
            self.cursor_row = self.rows - 1;
        } else {
            self.cursor_row += 1;
        }
    }

    pub fn println(&mut self, s: &str) {
        self.write(s);
        self.newline();
    }

    /// Text of one row, trailing spaces trimmed. Handy for tests.
    pub fn row_text(&self, row: usize) -> String {
        let s: String = self.cells[row * self.cols..(row + 1) * self.cols].iter().map(|c| c.ch).collect();
        s.trim_end().to_string()
    }

    pub fn to_text(&self) -> String {
        (0..self.rows).map(|r| self.row_text(r)).collect::<Vec<_>>().join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_wraps_and_scrolls() {
        let mut g = TextGrid::new(5, 2);
        g.write("abcdefg");
        assert_eq!(g.row_text(0), "abcde");
        assert_eq!(g.row_text(1), "fg");
        g.println("");
        g.write("x");
        assert_eq!(g.row_text(0), "fg");
        assert_eq!(g.row_text(1), "x");
    }

    #[test]
    fn clone_and_clear_keep_cell_scale() {
        let mut g = TextGrid::for_pixels(1280, 800);
        g.clear();
        assert_eq!(g.cell_scale(), 2);
        assert_eq!(g.clone().cell_size(), (16, 32));
    }

    #[test]
    fn for_pixels_keeps_80x25_at_every_16_10_preset() {
        for (w, h, s) in [(640, 400, 1), (1280, 800, 2), (1920, 1200, 3), (2560, 1600, 4)] {
            let g = TextGrid::for_pixels(w, h);
            assert_eq!((g.cols(), g.rows(), g.cell_scale()), (80, 25, s), "{w}x{h}");
        }
        let g = TextGrid::for_pixels(1920, 1080);
        assert_eq!((g.cols(), g.rows()), (80, 22));
        let g = TextGrid::for_pixels(320, 240);
        assert_eq!((g.cols(), g.rows(), g.cell_scale()), (40, 15, 1));
    }
}
