//! Shell/editor color themes (FR-C9).

use crate::color::Color;

#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub name: &'static str,
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
    // Syntax highlighting
    pub syn_keyword: Color,
    pub syn_string: Color,
    pub syn_number: Color,
    pub syn_comment: Color,
    pub syn_builtin: Color,
}

impl Theme {
    pub fn by_name(name: &str) -> Option<&'static Theme> {
        THEMES.iter().find(|t| t.name.eq_ignore_ascii_case(name))
    }

    pub fn names() -> impl Iterator<Item = &'static str> {
        THEMES.iter().map(|t| t.name)
    }

    pub fn default_theme() -> &'static Theme {
        &THEMES[0]
    }
}

pub static THEMES: [Theme; 4] = [
    Theme {
        name: "amber",
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
        syn_keyword: Color::rgb(255, 230, 150),
        syn_string: Color::rgb(255, 150, 80),
        syn_number: Color::rgb(255, 220, 190),
        syn_comment: Color::rgb(150, 100, 20),
        syn_builtin: Color::rgb(255, 200, 100),
    },
    Theme {
        name: "phosphor",
        bg: Color::rgb(6, 14, 8),
        fg: Color::rgb(80, 250, 123),
        dim: Color::rgb(40, 120, 60),
        accent: Color::rgb(180, 255, 200),
        error: Color::rgb(255, 110, 90),
        status_bg: Color::rgb(30, 90, 45),
        status_fg: Color::rgb(220, 255, 230),
        selection_bg: Color::rgb(30, 90, 45),
        menu_bg: Color::rgb(12, 40, 20),
        menu_fg: Color::rgb(170, 255, 190),
        menu_hl_bg: Color::rgb(80, 250, 123),
        menu_hl_fg: Color::rgb(6, 14, 8),
        syn_keyword: Color::rgb(180, 255, 200),
        syn_string: Color::rgb(255, 220, 120),
        syn_number: Color::rgb(160, 230, 255),
        syn_comment: Color::rgb(60, 140, 80),
        syn_builtin: Color::rgb(120, 220, 255),
    },
    Theme {
        name: "blue",
        bg: Color::rgb(0, 0, 170),
        fg: Color::rgb(255, 255, 255),
        dim: Color::rgb(130, 130, 255),
        accent: Color::rgb(255, 255, 85),
        error: Color::rgb(255, 85, 85),
        status_bg: Color::rgb(170, 170, 170),
        status_fg: Color::rgb(0, 0, 0),
        selection_bg: Color::rgb(85, 85, 255),
        menu_bg: Color::rgb(170, 170, 170),
        menu_fg: Color::rgb(0, 0, 0),
        menu_hl_bg: Color::rgb(0, 0, 0),
        menu_hl_fg: Color::rgb(255, 255, 255),
        syn_keyword: Color::rgb(255, 255, 85),
        syn_string: Color::rgb(85, 255, 255),
        syn_number: Color::rgb(85, 255, 85),
        syn_comment: Color::rgb(170, 170, 170),
        syn_builtin: Color::rgb(255, 85, 255),
    },
    Theme {
        name: "lcd",
        bg: Color::rgb(170, 190, 150),
        fg: Color::rgb(20, 30, 20),
        dim: Color::rgb(90, 110, 80),
        accent: Color::rgb(0, 0, 0),
        error: Color::rgb(120, 20, 20),
        status_bg: Color::rgb(20, 30, 20),
        status_fg: Color::rgb(170, 190, 150),
        selection_bg: Color::rgb(120, 140, 100),
        menu_bg: Color::rgb(140, 160, 120),
        menu_fg: Color::rgb(20, 30, 20),
        menu_hl_bg: Color::rgb(20, 30, 20),
        menu_hl_fg: Color::rgb(170, 190, 150),
        syn_keyword: Color::rgb(0, 0, 0),
        syn_string: Color::rgb(60, 60, 110),
        syn_number: Color::rgb(60, 90, 60),
        syn_comment: Color::rgb(100, 120, 90),
        syn_builtin: Color::rgb(40, 40, 80),
    },
];
