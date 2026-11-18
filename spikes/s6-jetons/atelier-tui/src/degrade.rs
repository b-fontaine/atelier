//! Détection des capacités couleur et dégradation truecolor → 256 → 16 → aucune.
use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ColorSupport {
    /// `NO_COLOR` : aucune couleur (les modificateurs restent permis).
    None,
    Ansi16,
    Ansi256,
    TrueColor,
}

impl ColorSupport {
    /// Détection depuis un accesseur d'environnement (injectable, donc testable sans toucher à l'env du processus).
    /// Priorité : `NO_COLOR` (non vide, cf. no-color.org) > `COLORTERM` > `TERM` > 16 couleurs.
    pub fn detect_with(get: impl Fn(&str) -> Option<String>) -> Self {
        if get("NO_COLOR").is_some_and(|v| !v.is_empty()) {
            return Self::None;
        }
        if get("COLORTERM").is_some_and(|v| matches!(v.to_ascii_lowercase().as_str(), "truecolor" | "24bit")) {
            return Self::TrueColor;
        }
        if get("TERM").is_some_and(|v| v.contains("256color")) {
            return Self::Ansi256;
        }
        Self::Ansi16
    }

    pub fn detect() -> Self {
        Self::detect_with(|k| std::env::var(k).ok())
    }
}

fn dist2(a: (u8, u8, u8), b: (u8, u8, u8)) -> u32 {
    let d = |x: u8, y: u8| (i32::from(x) - i32::from(y)).pow(2) as u32;
    d(a.0, b.0) + d(a.1, b.1) + d(a.2, b.2)
}

/// Paliers du cube 6×6×6 de la palette xterm 256.
const CUBE: [u8; 6] = [0, 95, 135, 175, 215, 255];

/// Index xterm-256 le plus proche (cube 16..=231 ou rampe de gris 232..=255).
pub fn nearest_256(r: u8, g: u8, b: u8) -> u8 {
    let idx = |v: u8| (0..6).min_by_key(|&i| (i32::from(CUBE[i]) - i32::from(v)).abs()).unwrap_or(0);
    let (ri, gi, bi) = (idx(r), idx(g), idx(b));
    let cube = (CUBE[ri], CUBE[gi], CUBE[bi]);
    let cube_index = 16 + 36 * ri as u8 + 6 * gi as u8 + bi as u8;
    // Rampe de gris : 8, 18, …, 238
    let avg = ((u16::from(r) + u16::from(g) + u16::from(b)) / 3) as u8;
    let gi = (0..24u8).min_by_key(|&i| (i32::from(8 + 10 * i) - i32::from(avg)).abs()).unwrap_or(0);
    let gv = 8 + 10 * gi;
    if dist2((r, g, b), (gv, gv, gv)) < dist2((r, g, b), cube) { 232 + gi } else { cube_index }
}

/// Les 16 couleurs ANSI avec leurs valeurs xterm par défaut.
const ANSI16: [((u8, u8, u8), Color); 16] = [
    ((0, 0, 0), Color::Black),
    ((205, 0, 0), Color::Red),
    ((0, 205, 0), Color::Green),
    ((205, 205, 0), Color::Yellow),
    ((0, 0, 238), Color::Blue),
    ((205, 0, 205), Color::Magenta),
    ((0, 205, 205), Color::Cyan),
    ((229, 229, 229), Color::Gray),
    ((127, 127, 127), Color::DarkGray),
    ((255, 0, 0), Color::LightRed),
    ((0, 255, 0), Color::LightGreen),
    ((255, 255, 0), Color::LightYellow),
    ((92, 92, 255), Color::LightBlue),
    ((255, 0, 255), Color::LightMagenta),
    ((0, 255, 255), Color::LightCyan),
    ((255, 255, 255), Color::White),
];

pub fn nearest_16(r: u8, g: u8, b: u8) -> Color {
    ANSI16.iter().min_by_key(|(c, _)| dist2((r, g, b), *c)).map_or(Color::Reset, |(_, n)| *n)
}

/// Dégrade une couleur selon la capacité du terminal. Seules les `Color::Rgb` sont converties.
pub fn degrade(color: Color, support: ColorSupport) -> Color {
    let Color::Rgb(r, g, b) = color else {
        return if support == ColorSupport::None { Color::Reset } else { color };
    };
    match support {
        ColorSupport::TrueColor => color,
        ColorSupport::Ansi256 => Color::Indexed(nearest_256(r, g, b)),
        ColorSupport::Ansi16 => nearest_16(r, g, b),
        ColorSupport::None => Color::Reset,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let m: HashMap<String, String> = pairs.iter().map(|(k, v)| ((*k).into(), (*v).into())).collect();
        move |k| m.get(k).cloned()
    }

    #[test]
    fn colorterm_truecolor_et_24bit() {
        assert_eq!(ColorSupport::detect_with(env(&[("COLORTERM", "truecolor")])), ColorSupport::TrueColor);
        assert_eq!(ColorSupport::detect_with(env(&[("COLORTERM", "24bit")])), ColorSupport::TrueColor);
    }

    #[test]
    fn term_256color_sans_colorterm() {
        assert_eq!(ColorSupport::detect_with(env(&[("TERM", "xterm-256color")])), ColorSupport::Ansi256);
    }

    #[test]
    fn repli_16_couleurs() {
        assert_eq!(ColorSupport::detect_with(env(&[("TERM", "xterm")])), ColorSupport::Ansi16);
        assert_eq!(ColorSupport::detect_with(env(&[])), ColorSupport::Ansi16);
    }

    #[test]
    fn no_color_prime_sur_colorterm() {
        let e = env(&[("NO_COLOR", "1"), ("COLORTERM", "truecolor")]);
        assert_eq!(ColorSupport::detect_with(e), ColorSupport::None);
    }

    #[test]
    fn no_color_vide_est_ignore() {
        let e = env(&[("NO_COLOR", ""), ("COLORTERM", "truecolor")]);
        assert_eq!(ColorSupport::detect_with(e), ColorSupport::TrueColor);
    }

    #[test]
    fn truecolor_inchange() {
        assert_eq!(degrade(Color::Rgb(1, 2, 3), ColorSupport::TrueColor), Color::Rgb(1, 2, 3));
    }

    #[test]
    fn index_256_connus() {
        assert_eq!(nearest_256(255, 0, 0), 196);
        assert_eq!(nearest_256(0, 0, 0), 16);
        assert_eq!(nearest_256(255, 255, 255), 231);
        assert_eq!(nearest_256(128, 128, 128), 244); // rampe de gris
    }

    #[test]
    fn degrade_256_et_16() {
        assert_eq!(degrade(Color::Rgb(255, 0, 0), ColorSupport::Ansi256), Color::Indexed(196));
        assert_eq!(degrade(Color::Rgb(250, 10, 10), ColorSupport::Ansi16), Color::LightRed);
        assert_eq!(degrade(Color::Rgb(15, 23, 42), ColorSupport::Ansi16), Color::Black);
    }

    #[test]
    fn aucune_couleur() {
        assert_eq!(degrade(Color::Rgb(1, 2, 3), ColorSupport::None), Color::Reset);
        assert_eq!(degrade(Color::Red, ColorSupport::None), Color::Reset);
    }
}
