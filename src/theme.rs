use serde::Deserialize;

/// Colour roles. Sections follow lualine: `a` is the outermost segment of a
/// side, `b` the next, everything further in sits on the bar itself.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Colors {
    pub bar_bg: String,
    pub bar_fg: String,
    pub muted: String,
    pub accent: String,
    pub a_bg: String,
    pub a_fg: String,
    pub b_bg: String,
    pub b_fg: String,
}

/// The built-in `wmux` theme. The bar is the terminal's own background, so it
/// works on light and dark terminals alike.
impl Default for Colors {
    fn default() -> Self {
        Self {
            bar_bg: "default".into(),
            bar_fg: "default".into(),
            muted: "#7f849c".into(),
            accent: "#89b4fa".into(),
            a_bg: "#89b4fa".into(),
            a_fg: "#1e1e2e".into(),
            b_bg: "#45475a".into(),
            b_fg: "#cdd6f4".into(),
        }
    }
}

impl Colors {
    /// Values are pasted into `#[fg=…]` and conditional branches, where `,` `]`
    /// and `}` would break the format; tmux colours never need them.
    pub fn validate(&self) -> Result<(), String> {
        for (key, value) in [
            ("bar_bg", &self.bar_bg),
            ("bar_fg", &self.bar_fg),
            ("muted", &self.muted),
            ("accent", &self.accent),
            ("a_bg", &self.a_bg),
            ("a_fg", &self.a_fg),
            ("b_bg", &self.b_bg),
            ("b_fg", &self.b_fg),
        ] {
            if value.is_empty() || !value.chars().all(|c| c.is_ascii_alphanumeric() || c == '#') {
                return Err(format!(
                    "colors.{key}: {value:?} is not a tmux colour (e.g. \"#89b4fa\", \"blue\", \"colour75\")"
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Separators {
    Powerline,
    Round,
    Plain,
}

/// `left` points right (between left-side segments), `right` points left.
/// `*_thin` separates neighbours that share a background.
pub struct Glyphs {
    pub left: &'static str,
    pub left_thin: &'static str,
    pub right: &'static str,
    pub right_thin: &'static str,
    pub branch: &'static str,
}

impl Separators {
    /// Without a Nerd Font every style falls back to plain colour blocks.
    pub fn glyphs(self, nerdfont: bool) -> Glyphs {
        let (left, left_thin, right, right_thin) = match (self, nerdfont) {
            (Self::Powerline, true) => ("\u{e0b0}", "\u{e0b1}", "\u{e0b2}", "\u{e0b3}"),
            (Self::Round, true) => ("\u{e0b4}", "\u{e0b5}", "\u{e0b6}", "\u{e0b7}"),
            _ => ("", "│", "", "│"),
        };
        let branch = if nerdfont { "\u{e0a0}" } else { "⎇" };
        Glyphs {
            left,
            left_thin,
            right,
            right_thin,
            branch,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_colors_are_valid() {
        Colors::default().validate().unwrap();
    }

    #[test]
    fn colors_reject_format_syntax() {
        for bad in ["", "red,bold", "#fff]", "a}b", "x y"] {
            let colors = Colors {
                accent: bad.into(),
                ..Colors::default()
            };
            assert!(colors.validate().is_err(), "{bad:?}");
        }
    }

    #[test]
    fn no_nerdfont_means_plain() {
        let g = Separators::Powerline.glyphs(false);
        assert_eq!((g.left, g.right, g.branch), ("", "", "⎇"));
    }
}
