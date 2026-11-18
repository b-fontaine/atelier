// GÉNÉRÉ par Style Dictionary depuis tokens/*.tokens.json — NE PAS ÉDITER.
#![allow(missing_docs)]

use ratatui::style::{Color, Modifier};

/// Palette sémantique d'un mode (couleurs truecolor + modificateurs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub surface: Color,
    pub texte: Color,
    pub accent: Color,
    pub danger: Color,
    pub emphasis: Modifier,
}

pub const LIGHT_SURFACE: Color = Color::Rgb(248, 250, 252);
pub const LIGHT_TEXTE: Color = Color::Rgb(15, 23, 42);
pub const LIGHT_ACCENT: Color = Color::Rgb(29, 78, 216);
pub const LIGHT_DANGER: Color = Color::Rgb(239, 68, 68);

pub const LIGHT: Palette = Palette {
    surface: LIGHT_SURFACE,
    texte: LIGHT_TEXTE,
    accent: LIGHT_ACCENT,
    danger: LIGHT_DANGER,
    emphasis: Modifier::BOLD,
};

pub const DARK_SURFACE: Color = Color::Rgb(15, 23, 42);
pub const DARK_TEXTE: Color = Color::Rgb(248, 250, 252);
pub const DARK_ACCENT: Color = Color::Rgb(96, 165, 250);
pub const DARK_DANGER: Color = Color::Rgb(248, 113, 113);

pub const DARK: Palette = Palette {
    surface: DARK_SURFACE,
    texte: DARK_TEXTE,
    accent: DARK_ACCENT,
    danger: DARK_DANGER,
    emphasis: Modifier::BOLD,
};
