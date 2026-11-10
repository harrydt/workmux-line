use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub sections: Sections,
    pub components: Components,
}

#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Components {
    pub agents: Agents,
}

/// Must match workmux's `status_icons`; defaults are workmux's defaults.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Agents {
    pub waiting: String,
    pub done: String,
}

impl Default for Agents {
    fn default() -> Self {
        Self {
            waiting: "💬".into(),
            done: "✅".into(),
        }
    }
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
            right: vec![Component::Agents, Component::Clock],
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Component {
    Session,
    Agents,
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

pub fn parse(text: &str) -> Result<Config, String> {
    let cfg: Config = toml::from_str(text).map_err(|e| e.to_string())?;
    let agents = &cfg.components.agents;
    for (key, icon) in [("waiting", &agents.waiting), ("done", &agents.done)] {
        // An empty icon would equal the empty status of every idle window.
        if icon.is_empty() || icon.chars().any(char::is_control) {
            return Err(format!(
                "components.agents.{key}: must be non-empty, without control characters"
            ));
        }
    }
    Ok(cfg)
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
    fn agent_icons_override_and_validate() {
        let cfg = parse("[components.agents]\nwaiting = \"W\"").unwrap();
        assert_eq!(cfg.components.agents.waiting, "W");
        assert_eq!(cfg.components.agents.done, "✅");

        let err = parse("[components.agents]\ndone = \"\"").unwrap_err();
        assert!(err.contains("components.agents.done"), "{err}");
        assert!(parse("[components.agents]\ndone = \"a\\nb\"").is_err());
    }

    #[test]
    fn missing_file_is_default() {
        let cfg = load(Path::new("/nonexistent/wmux-line/config.toml")).unwrap();
        assert_eq!(cfg, Config::default());
    }
}
