use crate::config::Component;

/// The tmux format a component renders as.
pub fn render(component: Component) -> &'static str {
    match component {
        Component::Session => "#S",
        Component::Clock => "%H:%M",
    }
}
