//! Shell/editor color themes (FR-C9).

use crate::color::Color;

#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    // pub name: &'static str,
    pub bg: Color,
    pub fg: Color,
    pub dim: Color,
    pub accent: Color,
    pub error: Color,
    pub status_bg: Color,
    pub status_fg: Color,
    pub selection_bg: Color,
    pub menu_bg: Color,
    pub menu_fg: Color,
    pub menu_hl_bg: Color,
    pub menu_hl_fg: Color,
}

pub const DEFAULT_THEME: Theme = Theme {
    // name: "amber",
    bg: Color::rgb(16, 10, 2),
    fg: Color::rgb(255, 176, 0),
    dim: Color::rgb(140, 90, 0),
    accent: Color::rgb(255, 230, 150),
    error: Color::rgb(255, 90, 60),
    status_bg: Color::rgb(120, 80, 0),
    status_fg: Color::rgb(255, 240, 200),
    selection_bg: Color::rgb(110, 70, 0),
    menu_bg: Color::rgb(50, 32, 4),
    menu_fg: Color::rgb(255, 210, 120),
    menu_hl_bg: Color::rgb(255, 176, 0),
    menu_hl_fg: Color::rgb(16, 10, 2),
};
