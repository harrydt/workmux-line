use crate::components::{self, Segment};
use crate::config::{Component, Config};
use crate::theme::{Colors, Glyphs};

/// tmux's defaults (10 left / 40 right) truncate real content.
const SECTION_LENGTH: u32 = 100;

/// A tmux config snippet, suitable for `tmux source-file -`.
pub fn generate(cfg: &Config) -> String {
    let c = &cfg.colors;
    let g = cfg.separators().glyphs(cfg.nerdfont);
    let agents = &cfg.components.agents;
    let left = left(&styled(cfg, cfg.left(), &g, false), c, &g);
    let right = right(&styled(cfg, cfg.right(), &g, true), c, &g);
    [
        ("@wmux_line_waiting", agents.waiting.clone()),
        ("@wmux_line_done", agents.done.clone()),
        ("status-style", format!("bg={},fg={}", c.bar_bg, c.bar_fg)),
        ("window-status-style", format!("fg={}", c.muted)),
        (
            "window-status-current-style",
            format!("fg={},bold", c.accent),
        ),
        ("status-left", left),
        ("status-left-length", SECTION_LENGTH.to_string()),
        ("status-right", right),
        ("status-right-length", SECTION_LENGTH.to_string()),
    ]
    .iter()
    .map(|(option, value)| format!("set -g {option} {}\n", quote(value)))
    .collect()
}

struct Styled {
    seg: Segment,
    fg: String,
    bg: String,
}

/// Colours by distance from the outer edge (lualine's a, b, then the bar),
/// and `min_width` folded into each segment's visibility.
fn styled(cfg: &Config, section: &[Component], g: &Glyphs, from_end: bool) -> Vec<Styled> {
    let c = &cfg.colors;
    section
        .iter()
        .enumerate()
        .map(|(i, &component)| {
            let mut seg = components::render(component, g);
            let min = cfg.components.min_width(component);
            if min > 0 {
                let wide = format!("#{{e|>=:#{{client_width}},{min}}}");
                seg.visible = Some(match seg.visible {
                    Some(v) => format!("#{{&&:{v},{wide}}}"),
                    None => wide,
                });
            }
            let tier = if from_end { section.len() - 1 - i } else { i };
            let (fg, bg) = match tier {
                0 => (&c.a_fg, &c.a_bg),
                1 => (&c.b_fg, &c.b_bg),
                _ => (&c.bar_fg, &c.bar_bg),
            };
            Styled {
                seg,
                fg: fg.clone(),
                bg: bg.clone(),
            }
        })
        .collect()
}

/// Each segment closes with a separator into the next *visible* segment's
/// background, chosen at render time by nesting the following segments'
/// visibility conditions. The last visible one closes into the bar, plus the
/// gap before the window list, so a fully hidden side leaves nothing behind.
fn left(segs: &[Styled], c: &Colors, g: &Glyphs) -> String {
    let mut out = String::new();
    for (i, s) in segs.iter().enumerate() {
        let edge = if s.bg == c.bar_bg {
            String::new()
        } else {
            arrow(&s.bg, &c.bar_bg, g.left)
        };
        let mut sep = edge + "#[default] ";
        for next in segs[i + 1..].iter().rev() {
            let t = if next.bg == s.bg {
                format!("#[fg={}]{}", c.muted, g.left_thin)
            } else {
                arrow(&s.bg, &next.bg, g.left)
            };
            sep = choose(&next.seg.visible, t, sep);
        }
        out += &when(
            &s.seg.visible,
            &format!("{} {} {sep}", style(&s.fg, &s.bg), s.seg.content),
        );
    }
    out
}

/// Mirror of `left`: each segment opens with a separator out of the previous
/// visible segment's background (or the bar's).
fn right(segs: &[Styled], c: &Colors, g: &Glyphs) -> String {
    let mut out = String::new();
    for (i, s) in segs.iter().enumerate() {
        let mut sep = if s.bg == c.bar_bg {
            String::new()
        } else {
            arrow(&s.bg, &c.bar_bg, g.right)
        };
        // Forwards, so the nearest previous segment ends up outermost and wins.
        for prev in &segs[..i] {
            let t = if prev.bg == s.bg {
                format!("#[fg={}]{}", c.muted, g.right_thin)
            } else {
                arrow(&s.bg, &prev.bg, g.right)
            };
            sep = choose(&prev.seg.visible, t, sep);
        }
        out += &when(
            &s.seg.visible,
            &format!("{sep}{} {} ", style(&s.fg, &s.bg), s.seg.content),
        );
    }
    out
}

/// A glyph drawn in `fg` over `bg`. Separate `#[…]` blocks, because a comma
/// inside a style would split an enclosing conditional.
fn arrow(fg: &str, bg: &str, glyph: &str) -> String {
    format!("#[fg={fg}]#[bg={bg}]{glyph}")
}

fn style(fg: &str, bg: &str) -> String {
    format!("#[fg={fg}]#[bg={bg}]")
}

/// `then` when `visible` holds; an always-visible segment makes `then` final.
fn choose(visible: &Option<String>, then: String, otherwise: String) -> String {
    match visible {
        None => then,
        Some(v) => format!("#{{?{v},{then},{otherwise}}}"),
    }
}

fn when(visible: &Option<String>, body: &str) -> String {
    choose(visible, body.to_string(), String::new())
}

/// Single-quote for tmux's config parser: nothing inside is expanded, and a
/// literal `'` is spliced in as `"'"`, which tmux concatenates like a shell.
fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r#"'"'"'"#))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Sections;

    fn cfg(left: &[Component], right: &[Component]) -> Config {
        Config {
            sections: Sections {
                left: Some(left.to_vec()),
                right: Some(right.to_vec()),
            },
            ..Config::default()
        }
    }

    fn option<'a>(out: &'a str, name: &str) -> &'a str {
        let prefix = format!("set -g {name} '");
        let line = out.lines().find(|l| l.starts_with(&prefix)).unwrap();
        &line[prefix.len()..line.len() - 1]
    }

    #[test]
    fn default_output_sets_every_option_once() {
        let out = generate(&Config::default());
        assert_eq!(option(&out, "@wmux_line_waiting"), "💬");
        assert_eq!(option(&out, "status-style"), "bg=default,fg=default");
        assert_eq!(out.lines().count(), 9);
    }

    #[test]
    fn empty_sections_render_nothing() {
        let out = generate(&cfg(&[], &[]));
        assert_eq!(option(&out, "status-left"), "");
        assert_eq!(option(&out, "status-right"), "");
    }

    #[test]
    fn always_visible_left_segments_chain_directly() {
        let out = generate(&cfg(&[Component::Session, Component::Session], &[]));
        assert_eq!(
            option(&out, "status-left"),
            "#[fg=#1e1e2e]#[bg=#89b4fa] #S #[fg=#89b4fa]#[bg=#45475a]\u{e0b4}\
             #[fg=#cdd6f4]#[bg=#45475a] #S #[fg=#45475a]#[bg=default]\u{e0b4}\
             #[default] "
        );
    }

    #[test]
    fn min_width_wraps_visibility() {
        let out = generate(&cfg(&[Component::Clock], &[]));
        assert!(
            option(&out, "status-left").starts_with("#{?#{e|>=:#{client_width},80},"),
            "{out}"
        );
    }

    #[test]
    fn quote_escapes_single_quotes() {
        assert_eq!(quote("it's"), r#"'it'"'"'s'"#);
        assert_eq!(quote("$HOME ~ #{x} ; \\n"), "'$HOME ~ #{x} ; \\n'");
    }
}
