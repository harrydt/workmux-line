use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::theme::{Colors, Separators};

#[derive(Debug, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub preset: Preset,
    /// Mirrors workmux's own `nerdfont` setting; off means Unicode fallbacks.
    pub nerdfont: bool,
    /// Off: leave the status line alone and only publish `@wmux_line_agents`
    /// for embedding in your own (`#{E:@wmux_line_agents}`).
    pub statusline: bool,
    /// Overrides the preset's separator style.
    pub separators: Option<Separators>,
    pub sections: Sections,
    pub components: Components,
    pub colors: Colors,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            preset: Preset::Developer,
            nerdfont: true,
            statusline: true,
            separators: None,
            sections: Sections::default(),
            components: Components::default(),
            colors: Colors::default(),
        }
    }
}

impl Config {
    pub fn left(&self) -> &[Component] {
        self.sections.left.as_deref().unwrap_or(self.preset.left())
    }

    pub fn right(&self) -> &[Component] {
        self.sections
            .right
            .as_deref()
            .unwrap_or(self.preset.right())
    }

    pub fn separators(&self) -> Separators {
        self.separators.unwrap_or(self.preset.separators())
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Preset {
    Minimal,
    Developer,
    Powerline,
}

impl Preset {
    fn left(self) -> &'static [Component] {
        match self {
            Self::Minimal => &[Component::Session],
            Self::Developer | Self::Powerline => &[Component::Session, Component::Branch],
        }
    }

    fn right(self) -> &'static [Component] {
        match self {
            Self::Minimal => &[Component::Agents, Component::Mode],
            Self::Developer | Self::Powerline => {
                &[Component::Agents, Component::Mode, Component::Clock]
            }
        }
    }

    fn separators(self) -> Separators {
        match self {
            Self::Minimal => Separators::Plain,
            Self::Developer => Separators::Round,
            Self::Powerline => Separators::Powerline,
        }
    }
}

/// Unset sides come from the preset.
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Sections {
    pub left: Option<Vec<Component>>,
    pub right: Option<Vec<Component>>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Component {
    Session,
    Branch,
    Agents,
    /// Prefix held / copy mode badge.
    Mode,
    Clock,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Components {
    pub session: Width,
    pub branch: Width,
    pub agents: Agents,
    pub mode: Width,
    pub clock: Width,
}

/// Narrow terminals drop the least essential components first.
impl Default for Components {
    fn default() -> Self {
        Self {
            session: Width::default(),
            branch: Width { min_width: 90 },
            agents: Agents::default(),
            mode: Width::default(),
            clock: Width { min_width: 80 },
        }
    }
}

impl Components {
    /// Hide below this client width; 0 means always shown.
    pub fn min_width(&self, component: Component) -> u16 {
        match component {
            Component::Session => self.session.min_width,
            Component::Branch => self.branch.min_width,
            Component::Agents => self.agents.min_width,
            Component::Mode => self.mode.min_width,
            Component::Clock => self.clock.min_width,
        }
    }
}

#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Width {
    pub min_width: u16,
}

/// Icons must match workmux's `status_icons`; defaults are workmux's defaults.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Agents {
    pub waiting: String,
    pub done: String,
    pub min_width: u16,
}

impl Default for Agents {
    fn default() -> Self {
        Self {
            waiting: "💬".into(),
            done: "✅".into(),
            min_width: 0,
        }
    }
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
    cfg.colors.validate()?;
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
    fn preset_supplies_layout_and_separators() {
        let cfg = parse("preset = \"minimal\"").unwrap();
        assert_eq!(cfg.left(), [Component::Session]);
        assert_eq!(cfg.right(), [Component::Agents, Component::Mode]);
        assert_eq!(cfg.separators(), Separators::Plain);
    }

    #[test]
    fn explicit_settings_override_preset() {
        let cfg = parse("preset = \"minimal\"\nseparators = \"powerline\"\n[sections]\nright = []")
            .unwrap();
        assert_eq!(cfg.left(), [Component::Session], "unset side stays preset");
        assert!(cfg.right().is_empty());
        assert_eq!(cfg.separators(), Separators::Powerline);
    }

    #[test]
    fn unknown_key_is_an_error_naming_the_key() {
        let err = parse("[sections]\nleft = []\nmiddle = []").unwrap_err();
        assert!(err.contains("middle"), "{err}");
    }

    #[test]
    fn unknown_component_is_an_error_naming_it() {
        let err = parse("[sections]\nleft = [\"battery\"]").unwrap_err();
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
    fn min_width_defaults_and_override() {
        let cfg = parse("[components.clock]\nmin_width = 120").unwrap();
        assert_eq!(cfg.components.min_width(Component::Clock), 120);
        assert_eq!(cfg.components.min_width(Component::Branch), 90);
        assert_eq!(cfg.components.min_width(Component::Session), 0);
    }

    #[test]
    fn bad_color_is_an_error_naming_it() {
        let err = parse("[colors]\naccent = \"red,bold\"").unwrap_err();
        assert!(err.contains("colors.accent"), "{err}");
    }

    #[test]
    fn missing_file_is_default() {
        let cfg = load(Path::new("/nonexistent/wmux-line/config.toml")).unwrap();
        assert_eq!(cfg, Config::default());
    }
}
