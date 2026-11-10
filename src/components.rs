use crate::config::Component;

/// The tmux format a component renders as.
pub fn render(component: Component) -> String {
    match component {
        Component::Session => "#S".into(),
        Component::Agents => agents(),
        Component::Clock => "%H:%M".into(),
    }
}

// Icons are compared and shown through tmux options (set by `generate`), never
// inlined: tmux substitutes option values verbatim, so icons need no escaping.
const WAITING: &str = "#{==:#{@workmux_status},#{@wmux_line_waiting}}";
const DONE: &str = "#{==:#{@workmux_status},#{@wmux_line_done}}";

/// Windows, across all sessions, whose agent is waiting or done and not yet
/// seen (workmux clears those icons on focus). Hidden when there are none,
/// `<icon> <worktree>` for one, per-status counts for more.
#[rustfmt::skip] // one line per format fragment reads better than one per token
fn agents() -> String {
    let attention = ["#{||:", WAITING, ",", DONE, "}"].concat();
    let total = count(&attention);
    let waiting = count(WAITING);
    let done = count(DONE);
    // The worktree is the active pane's directory name; session and window
    // names carry workmux's configurable prefix.
    let one = ["#{S:#{W:#{?", &attention, ",#{@workmux_status} #{b:pane_current_path},}}}"].concat();
    let many = [
        "#{?#{!=:", &waiting, ",0},#{@wmux_line_waiting} ", &waiting, ",}",
        "#{?#{&&:#{!=:", &waiting, ",0},#{!=:", &done, ",0}}, ,}",
        "#{?#{!=:", &done, ",0},#{@wmux_line_done} ", &done, ",}",
    ].concat();
    ["#{?#{==:", &total, ",0},,#{?#{==:", &total, ",1},", &one, ",", &many, "}}"].concat()
}

/// Number of windows on the server matching `cond` (one `x` per match).
fn count(cond: &str) -> String {
    ["#{n:#{S:#{W:#{?", cond, ",x,}}}}"].concat()
}
