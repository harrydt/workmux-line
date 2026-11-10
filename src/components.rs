use crate::config::Component;
use crate::theme::Glyphs;

/// A component as tmux formats: what it shows, and when (`None` = always).
/// `visible` must be a `#{…}` expression; tmux reads a bare word as a variable.
pub struct Segment {
    pub content: String,
    pub visible: Option<String>,
}

pub fn render(component: Component, glyphs: &Glyphs) -> Segment {
    match component {
        Component::Session => always("#S"),
        Component::Branch => branch(glyphs.branch),
        Component::Agents => Segment {
            content: agents(),
            visible: Some(["#{!=:", &count(&attention()), ",0}"].concat()),
        },
        Component::Mode => Segment {
            // Labels via options, like the agent icons: no escaping needed.
            content: "#{?client_prefix,#{@wmux_line_prefix},#{@wmux_line_copy}}".into(),
            visible: Some("#{||:#{client_prefix},#{==:#{pane_mode},copy-mode}}".into()),
        },
        Component::Clock => always("%H:%M"),
    }
}

fn always(content: &str) -> Segment {
    Segment {
        content: content.into(),
        visible: None,
    }
}

/// tmux runs this job asynchronously, caches it per client and command
/// string, and reruns it at most once a second, when the pane path changes or
/// on status-interval. Both uses below share one job.
const BRANCH_JOB: &str = "#(git -C #{q:pane_current_path} branch --show-current 2>/dev/null)";

/// Hidden outside a repository and on a detached HEAD (both print nothing).
fn branch(icon: &str) -> Segment {
    Segment {
        content: format!("{icon} {BRANCH_JOB}"),
        visible: Some(["#{!=:", BRANCH_JOB, ",}"].concat()),
    }
}

// Icons are compared and shown through tmux options (set by `generate`), never
// inlined: tmux substitutes option values verbatim, so icons need no escaping.
const WAITING: &str = "#{==:#{@workmux_status},#{@wmux_line_waiting}}";
const DONE: &str = "#{==:#{@workmux_status},#{@wmux_line_done}}";

fn attention() -> String {
    ["#{||:", WAITING, ",", DONE, "}"].concat()
}

/// Windows, across all sessions, whose agent is waiting or done and not yet
/// seen (workmux clears those icons on focus): `<icon> <worktree>` for one,
/// per-status counts for more. `visible` hides it when there are none.
#[rustfmt::skip] // one line per format fragment reads better than one per token
fn agents() -> String {
    let attention = attention();
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
    ["#{?#{==:", &total, ",1},", &one, ",", &many, "}"].concat()
}

/// Number of windows on the server matching `cond` (one `x` per match).
fn count(cond: &str) -> String {
    ["#{n:#{S:#{W:#{?", cond, ",x,}}}}"].concat()
}
