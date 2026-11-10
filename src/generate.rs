use crate::components;
use crate::config::{Component, Config};

/// tmux's defaults (10 left / 40 right) truncate real content.
const SECTION_LENGTH: u32 = 100;

/// A tmux config snippet, suitable for `tmux source-file -`.
pub fn generate(cfg: &Config) -> String {
    let mut out = String::new();
    for (option, section) in [
        ("status-left", &cfg.sections.left),
        ("status-right", &cfg.sections.right),
    ] {
        out += &format!("set -g {option} {}\n", quote(&section_format(section)));
        out += &format!("set -g {option}-length {SECTION_LENGTH}\n");
    }
    out
}

fn section_format(section: &[Component]) -> String {
    if section.is_empty() {
        return String::new();
    }
    let parts: Vec<_> = section.iter().map(|&c| components::render(c)).collect();
    format!(" {} ", parts.join(" "))
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

    #[test]
    fn default_output() {
        assert_eq!(
            generate(&Config::default()),
            "set -g status-left ' #S '\n\
             set -g status-left-length 100\n\
             set -g status-right ' %H:%M '\n\
             set -g status-right-length 100\n"
        );
    }

    #[test]
    fn empty_section_renders_nothing() {
        let cfg = Config {
            sections: Sections {
                left: vec![],
                right: vec![Component::Session, Component::Clock],
            },
        };
        let out = generate(&cfg);
        assert!(out.contains("set -g status-left ''\n"), "{out}");
        assert!(out.contains("set -g status-right ' #S %H:%M '\n"), "{out}");
    }

    #[test]
    fn quote_escapes_single_quotes() {
        assert_eq!(quote("it's"), r#"'it'"'"'s'"#);
        assert_eq!(quote("$HOME ~ #{x} ; \\n"), "'$HOME ~ #{x} ; \\n'");
    }
}
