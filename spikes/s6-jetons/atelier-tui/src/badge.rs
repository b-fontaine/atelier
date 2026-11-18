//! Atome `Badge` : étiquette courte colorée par les jetons sémantiques.
use crate::degrade::{ColorSupport, degrade};
use crate::generated::Palette;
use ratatui::{buffer::Buffer, layout::Rect, style::{Modifier, Style}, widgets::Widget};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Accent,
    Danger,
}

pub struct Badge<'a> {
    label: &'a str,
    kind: Kind,
    palette: &'a Palette,
    support: ColorSupport,
}

impl<'a> Badge<'a> {
    pub fn new(label: &'a str, kind: Kind, palette: &'a Palette, support: ColorSupport) -> Self {
        Self { label, kind, palette, support }
    }

    /// Texte `surface` sur fond `accent`/`danger`, en emphase. Sans couleur (`NO_COLOR`),
    /// l'inversion vidéo garde le badge distinguable.
    pub fn style(&self) -> Style {
        let bg = match self.kind {
            Kind::Accent => self.palette.accent,
            Kind::Danger => self.palette.danger,
        };
        let style = Style::new()
            .fg(degrade(self.palette.surface, self.support))
            .bg(degrade(bg, self.support))
            .add_modifier(self.palette.emphasis);
        if self.support == ColorSupport::None { Style::new().add_modifier(self.palette.emphasis | Modifier::REVERSED) } else { style }
    }
}

impl Widget for Badge<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        buf.set_string(area.x, area.y, format!(" {} ", self.label), self.style());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::{DARK, DARK_DANGER, LIGHT, LIGHT_ACCENT, LIGHT_SURFACE};
    use ratatui::{Terminal, backend::TestBackend, style::Color};

    fn rendu(badge: Badge<'_>) -> Buffer {
        let mut t = Terminal::new(TestBackend::new(10, 1)).unwrap();
        t.draw(|f| f.render_widget(badge, f.area())).unwrap();
        t.backend().buffer().clone()
    }

    #[test]
    fn couleurs_truecolor_issues_des_jetons() {
        let buf = rendu(Badge::new("ok", Kind::Accent, &LIGHT, ColorSupport::TrueColor));
        assert_eq!(buf[(1, 0)].symbol(), "o");
        assert_eq!(buf[(1, 0)].bg, LIGHT_ACCENT);
        assert_eq!(buf[(1, 0)].fg, LIGHT_SURFACE);
        assert_eq!(buf[(1, 0)].bg, Color::Rgb(29, 78, 216)); // valeur du fichier JSON (#1d4ed8)
        assert!(buf[(1, 0)].modifier.contains(Modifier::BOLD));
        assert_eq!(buf[(5, 0)].bg, Color::Reset); // hors du badge
    }

    #[test]
    fn mode_sombre_danger() {
        let buf = rendu(Badge::new("ko", Kind::Danger, &DARK, ColorSupport::TrueColor));
        assert_eq!(buf[(1, 0)].bg, DARK_DANGER);
    }

    #[test]
    fn degradation_256() {
        let buf = rendu(Badge::new("ok", Kind::Accent, &LIGHT, ColorSupport::Ansi256));
        assert!(matches!(buf[(1, 0)].bg, Color::Indexed(_)));
    }

    #[test]
    fn no_color_sans_couleur_mais_inverse_et_gras() {
        let buf = rendu(Badge::new("ok", Kind::Accent, &LIGHT, ColorSupport::None));
        let c = &buf[(1, 0)];
        assert_eq!((c.fg, c.bg), (Color::Reset, Color::Reset));
        assert!(c.modifier.contains(Modifier::REVERSED | Modifier::BOLD));
    }
}
