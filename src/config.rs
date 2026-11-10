use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub sections: Sections,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Sections {
    pub left: Vec<Component>,
    pub right: Vec<Component>,
}

impl Default for Sections {
    fn default() -> Self {
        Self {
            left: vec![Component::Session],
            right: vec![Component::Clock],
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Component {
    Session,
    Clock,
}

/// `$XDG_CONFIG_HOME/wmux-line/config.toml`, else `~/.config/wmux-line/config.toml`.
pub fn default_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("wmux-line/config.toml"))
}

/// A missing file means defaults; anything else unreadable or invalid is an error.
pub fn load(path: &Path) -> Result<Config, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => parse(&text).map_err(|e| format!("{}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

pub fn parse(text: &str) -> Result<Config, toml::de::Error> {
    toml::from_str(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_config_is_default() {
        assert_eq!(parse("").unwrap(), Config::default());
    }

    #[test]
    fn partial_sections_keep_other_defaults() {
        let cfg = parse("[sections]\nright = []").unwrap();
        assert_eq!(cfg.sections.left, vec![Component::Session]);
        assert!(cfg.sections.right.is_empty());
    }

    #[test]
    fn unknown_key_is_an_error_naming_the_key() {
        let err = parse("[sections]\nleft = []\nmiddle = []")
            .unwrap_err()
            .to_string();
        assert!(err.contains("middle"), "{err}");
    }

    #[test]
    fn unknown_component_is_an_error_naming_it() {
        let err = parse("[sections]\nleft = [\"battery\"]")
            .unwrap_err()
            .to_string();
        assert!(err.contains("battery"), "{err}");
    }

    #[test]
    fn missing_file_is_default() {
        let cfg = load(Path::new("/nonexistent/wmux-line/config.toml")).unwrap();
        assert_eq!(cfg, Config::default());
    }
}
