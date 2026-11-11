//! End to end: `workmux-line print` sourced into an isolated tmux server, with
//! workmux simulated by setting `@workmux_status` the way it does.
//!
//! Formats are evaluated through a control-mode client (`tmux -C`): unlike a
//! one-shot `tmux display`, it persists, so `#()` job results (cached per
//! client) become visible, and its width can be set for `min_width`.

use std::io::{BufRead, BufReader, Lines, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

/// The socket lives in `dir` because tmux leaves it behind after `kill-server`.
struct Tmux {
    dir: PathBuf,
    client: Child,
    input: ChildStdin,
    output: Lines<BufReader<ChildStdout>>,
}

impl Tmux {
    /// `None` when tmux isn't installed.
    fn start(test: &str, config: &str) -> Option<Self> {
        Command::new("tmux").arg("-V").output().ok()?;
        let dir =
            std::env::temp_dir().join(format!("workmux-line-test-{}-{test}", std::process::id()));
        std::fs::create_dir_all(dir.join("workmux-line")).unwrap();
        std::fs::write(dir.join("workmux-line/config.toml"), config).unwrap();

        let out = Command::new(env!("CARGO_BIN_EXE_workmux-line"))
            .arg("print")
            .env("XDG_CONFIG_HOME", &dir)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );

        let main = dir.join("main");
        std::fs::create_dir_all(&main).unwrap();
        run(
            &dir,
            &[
                "new-session",
                "-d",
                "-s",
                "main",
                "-c",
                main.to_str().unwrap(),
            ],
        );
        let mut source = tmux(&dir)
            .args(["source-file", "-"])
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        source.stdin.take().unwrap().write_all(&out.stdout).unwrap();
        assert!(source.wait().unwrap().success());

        let mut client = tmux(&dir)
            .args(["-C", "attach", "-t", "main"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let input = client.stdin.take().unwrap();
        let output = BufReader::new(client.stdout.take().unwrap()).lines();
        let mut tmux = Self {
            dir,
            client,
            input,
            output,
        };
        tmux.reply("attach"); // tmux answers the attach itself first
        tmux.command("refresh-client -C 200x50");
        Some(tmux)
    }

    /// One command through the control client; returns its output lines.
    fn command(&mut self, cmd: &str) -> String {
        writeln!(self.input, "{cmd}").unwrap();
        self.reply(cmd)
    }

    /// The next `%begin … %end` block, skipping notifications around it.
    fn reply(&mut self, cmd: &str) -> String {
        let mut body = Vec::new();
        let mut inside = false;
        for line in self.output.by_ref() {
            let line = line.unwrap();
            if line.starts_with("%begin") {
                inside = true;
            } else if inside && line.starts_with("%error") {
                panic!("tmux {cmd}: {}", body.join("\n"));
            } else if inside && line.starts_with("%end") {
                return body.join("\n");
            } else if inside {
                body.push(line);
            }
        }
        panic!("control client exited during {cmd}");
    }

    /// `fmt` expanded the way the status line expands it (`#{T:…}`: formats,
    /// then strftime) for the control client and its `main` session.
    fn eval(&mut self, fmt: &str) -> String {
        self.command(&format!("display -p -t main: {}", quote(fmt)))
    }

    /// Jobs start on first expansion and land later; poll until `want` holds.
    fn eval_until(&mut self, fmt: &str, want: impl Fn(&str) -> bool) -> String {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let got = self.eval(fmt);
            if want(&got) || Instant::now() > deadline {
                return got;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    fn width(&mut self, columns: u16) {
        self.command(&format!("refresh-client -C {columns}x50"));
    }

    /// A session whose pane sits in a directory named like a worktree.
    fn session(&mut self, name: &str) -> PathBuf {
        let wt = self.dir.join(name);
        std::fs::create_dir_all(&wt).unwrap();
        run(
            &self.dir,
            &["new-session", "-d", "-s", name, "-c", wt.to_str().unwrap()],
        );
        wt
    }

    fn status(&mut self, session: &str, icon: Option<&str>) {
        let target = format!("{session}:");
        match icon {
            Some(icon) => run(
                &self.dir,
                &["set", "-w", "-t", &target, "@workmux_status", icon],
            ),
            None => run(&self.dir, &["set", "-uw", "-t", &target, "@workmux_status"]),
        }
    }

    fn left(&mut self) -> String {
        self.eval("#{T:status-left}")
    }

    fn right(&mut self) -> String {
        self.eval("#{T:status-right}")
    }

    /// `workmux-line apply` with `config`, finding this server through `$TMUX`
    /// the way it does under `run-shell`.
    fn apply(&self, config: &str) -> std::process::Output {
        std::fs::write(self.dir.join("workmux-line/config.toml"), config).unwrap();
        let socket = self.dir.join("sock");
        Command::new(env!("CARGO_BIN_EXE_workmux-line"))
            .arg("apply")
            .env("XDG_CONFIG_HOME", &self.dir)
            .env("TMUX", format!("{},0,0", socket.display()))
            .output()
            .unwrap()
    }
}

impl Drop for Tmux {
    fn drop(&mut self) {
        let _ = tmux(&self.dir).arg("kill-server").output();
        let _ = self.client.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn tmux(dir: &std::path::Path) -> Command {
    let mut cmd = Command::new("tmux");
    cmd.env_remove("TMUX")
        .arg("-S")
        .arg(dir.join("sock"))
        .args(["-f", "/dev/null"]);
    cmd
}

fn run(dir: &std::path::Path, args: &[&str]) {
    let out = tmux(dir).args(args).output().unwrap();
    assert!(
        out.status.success(),
        "tmux {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r#"'"'"'"#))
}

/// What the user reads: the line with `#[…]` styles removed, trimmed.
fn text(rendered: &str) -> String {
    let mut out = String::new();
    let mut rest = rendered;
    while let Some(start) = rest.find("#[") {
        out += &rest[..start];
        rest = rest[start..].split_once(']').map_or("", |(_, after)| after);
    }
    out += rest;
    out.trim().to_string()
}

macro_rules! start {
    ($test:expr, $config:expr) => {
        match Tmux::start($test, $config) {
            Some(tmux) => tmux,
            None => {
                eprintln!("tmux not installed; skipping");
                return;
            }
        }
    };
}

const AGENTS_ONLY: &str = "separators = \"plain\"\n[sections]\nleft = []\nright = [\"agents\"]\n";

#[test]
fn agents_indicator_tracks_workmux_status() {
    let mut tmux = start!("agents", AGENTS_ONLY);
    for name in ["alpha", "beta", "gamma"] {
        tmux.session(name);
    }

    assert_eq!(tmux.right(), "", "nothing pending renders nothing at all");

    tmux.status("alpha", Some("🤖"));
    assert_eq!(tmux.right(), "", "working is not attention");

    tmux.status("beta", Some("💬"));
    assert_eq!(
        text(&tmux.right()),
        "💬 beta",
        "one agent shows its worktree"
    );

    tmux.status("gamma", Some("✅"));
    assert_eq!(text(&tmux.right()), "💬 1 ✅ 1");

    tmux.status("alpha", Some("💬"));
    assert_eq!(text(&tmux.right()), "💬 2 ✅ 1");

    tmux.status("gamma", None);
    tmux.status("alpha", None);
    assert_eq!(
        text(&tmux.right()),
        "💬 beta",
        "cleared (focused) agents drop out"
    );
}

#[test]
fn custom_icons_need_no_escaping() {
    // Commas, braces, tmux aliases (#S), strftime (%H), quotes and styles.
    let icon = "#[fg=#a6e3a1,bold]a,b}#S%H'q#[default]";
    let config = format!("{AGENTS_ONLY}[components.agents]\nwaiting = \"{icon}\"\n");
    let mut tmux = start!("icons", &config);
    tmux.session("alpha");
    tmux.session("beta");

    tmux.status("alpha", Some("💬"));
    assert_eq!(tmux.right(), "", "default icon no longer means waiting");

    tmux.status("alpha", Some(icon));
    assert!(
        tmux.right().contains(&format!("{icon} alpha")),
        "{}",
        tmux.right()
    );

    tmux.status("beta", Some(icon));
    assert!(
        tmux.right().contains(&format!("{icon} 2")),
        "{}",
        tmux.right()
    );
}

#[test]
fn hidden_segment_takes_its_separator_with_it() {
    let config =
        "separators = \"powerline\"\n[sections]\nleft = []\nright = [\"agents\", \"session\"]\n";
    let mut tmux = start!("separators", config);
    tmux.session("beta");
    let (a_bg, b_bg, arrow) = ("#89b4fa", "#45475a", "\u{e0b2}");

    // Agents hidden: session (tier a) opens straight from the bar.
    let right = tmux.right();
    assert_eq!(text(&right), format!("{arrow} main"));
    assert!(
        right.starts_with(&format!("#[fg={a_bg}]#[bg=default]{arrow}")),
        "{right}"
    );

    // Agents shown (tier b): bar → b, then b → a.
    tmux.status("beta", Some("💬"));
    let right = tmux.right();
    assert_eq!(text(&right), format!("{arrow} 💬 beta {arrow} main"));
    assert!(
        right.contains(&format!("#[fg={a_bg}]#[bg={b_bg}]{arrow}")),
        "{right}"
    );
}

#[test]
fn min_width_hides_on_narrow_clients() {
    let config =
        "[sections]\nleft = [\"session\"]\nright = []\n[components.session]\nmin_width = 100\n";
    let mut tmux = start!("width", config);

    assert!(text(&tmux.left()).contains("main"), "wide client shows it");
    tmux.width(80);
    assert_eq!(tmux.left(), "", "narrow client hides it, separators too");
}

#[test]
fn branch_shows_in_git_repos_only() {
    let config =
        "[sections]\nleft = [\"branch\"]\nright = []\n[components.branch]\nmin_width = 0\n";
    let mut tmux = start!("branch", config);
    let has_git = Command::new("git").arg("--version").output().is_ok();
    if !has_git {
        eprintln!("git not installed; skipping");
        return;
    }

    // `main` isn't a repo: hidden. The first expansion only starts the job, so
    // let it finish before trusting "".
    tmux.left();
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(tmux.left(), "");

    let repo = tmux.dir.join("main");
    let init = Command::new("git")
        .args(["init", "-q", "-b", "feat-x"])
        .current_dir(&repo)
        .output()
        .unwrap();
    assert!(init.status.success());
    let left = tmux.eval_until("#{T:status-left}", |s| s.contains("feat-x"));
    assert_eq!(text(&left), "\u{e0a0} feat-x \u{e0b4}");
}

#[test]
fn apply_updates_the_running_server() {
    let mut tmux = start!("apply", "[sections]\nleft = []\nright = []\n");
    assert_eq!(tmux.left(), "");

    let out = tmux.apply("[sections]\nleft = [\"session\"]\nright = []\n");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(text(&tmux.left()).contains("main"), "{}", tmux.left());
}

#[test]
fn apply_with_broken_config_leaves_tmux_untouched() {
    let mut tmux = start!(
        "apply-broken",
        "[sections]\nleft = [\"session\"]\nright = []\n"
    );
    let before = tmux.left();

    let out = tmux.apply("[sections]\nleft = [\"battery\"]\n");
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("battery"), "{err}");
    assert_eq!(tmux.left(), before);
}

#[test]
fn apply_without_a_server_says_so() {
    let dir =
        std::env::temp_dir().join(format!("workmux-line-test-{}-noserver", std::process::id()));
    let out = Command::new(env!("CARGO_BIN_EXE_workmux-line"))
        .arg("apply")
        .env("XDG_CONFIG_HOME", &dir) // no config: defaults
        .env("TMUX", format!("{},0,0", dir.join("sock").display()))
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.starts_with("workmux-line: tmux: "), "{err}");
}

#[test]
fn module_embeds_in_a_foreign_status_line() {
    let mut tmux = start!("module", "statusline = false\n");
    tmux.session("alpha");
    tmux.session("beta");
    let theirs = "#[fg=red]THEIRS #{E:@workmux_line_agents}| %%";
    run(&tmux.dir, &["set", "-g", "status-right", theirs]);
    assert_eq!(
        tmux.eval("#{status-left}"),
        "[#{session_name}] ",
        "tmux default kept"
    );

    assert_eq!(tmux.right(), "#[fg=red]THEIRS | %");
    tmux.status("alpha", Some("💬"));
    assert_eq!(tmux.right(), "#[fg=red]THEIRS 💬 alpha| %");
    tmux.status("beta", Some("✅"));
    assert_eq!(tmux.right(), "#[fg=red]THEIRS 💬 1 ✅ 1| %");
}

#[test]
fn mode_badge_shows_prefix_and_copy_mode() {
    let config = "[sections]\nleft = []\nright = [\"mode\"]\n";
    let mut tmux = start!("mode", config);

    assert_eq!(tmux.right(), "", "hidden normally");

    // What pressing the prefix does: the client enters the prefix key table.
    tmux.command("switch-client -T prefix");
    assert_eq!(
        text(&tmux.right()),
        "\u{e0b6} \u{f11c}",
        "keyboard icon by default"
    );
    tmux.command("switch-client -T root");
    assert_eq!(tmux.right(), "");

    tmux.command("copy-mode -t main:");
    assert_eq!(text(&tmux.right()), "\u{e0b6} COPY");
    tmux.command("send-keys -X -t main: cancel");
    assert_eq!(tmux.right(), "");
}

#[test]
fn mode_labels_are_configurable() {
    let config = "[sections]\nleft = []\nright = [\"mode\"]\n[components.mode]\nprefix = \"P,}#S\"\ncopy = \"📋\"\n";
    let mut tmux = start!("mode-labels", config);

    tmux.command("switch-client -T prefix");
    assert_eq!(
        text(&tmux.right()),
        "\u{e0b6} P,}#S",
        "verbatim, no escaping"
    );
    tmux.command("switch-client -T root");
    tmux.command("copy-mode -t main:");
    assert_eq!(text(&tmux.right()), "\u{e0b6} 📋");
}
