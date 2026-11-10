//! End to end: `wmux-line print` sourced into an isolated tmux server, with
//! workmux simulated by setting `@workmux_status` the way it does.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// The socket lives in `dir` because tmux leaves it behind after `kill-server`.
struct Tmux {
    dir: PathBuf,
}

impl Tmux {
    /// `None` when tmux isn't installed.
    fn start(test: &str, config: &str) -> Option<Self> {
        Command::new("tmux").arg("-V").output().ok()?;
        let dir =
            std::env::temp_dir().join(format!("wmux-line-test-{}-{test}", std::process::id()));
        std::fs::create_dir_all(dir.join("wmux-line")).unwrap();
        std::fs::write(dir.join("wmux-line/config.toml"), config).unwrap();
        let tmux = Self { dir };

        let out = Command::new(env!("CARGO_BIN_EXE_wmux-line"))
            .arg("print")
            .env("XDG_CONFIG_HOME", &tmux.dir)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        tmux.session("main");
        let mut source = tmux
            .cmd(&["source-file", "-"])
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        source.stdin.take().unwrap().write_all(&out.stdout).unwrap();
        assert!(source.wait().unwrap().success());
        Some(tmux)
    }

    fn cmd(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new("tmux");
        cmd.env_remove("TMUX")
            .arg("-S")
            .arg(self.dir.join("sock"))
            .args(["-f", "/dev/null"])
            .args(args);
        cmd
    }

    fn run(&self, args: &[&str]) -> String {
        let out = self.cmd(args).output().unwrap();
        assert!(
            out.status.success(),
            "tmux {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap()
    }

    /// A session whose pane sits in a directory named like a worktree.
    fn session(&self, name: &str) {
        let wt = self.dir.join(name);
        std::fs::create_dir_all(&wt).unwrap();
        self.run(&["new-session", "-d", "-s", name, "-c", wt.to_str().unwrap()]);
    }

    fn status(&self, session: &str, icon: Option<&str>) {
        let target = format!("{session}:");
        match icon {
            Some(icon) => self.run(&["set", "-w", "-t", &target, "@workmux_status", icon]),
            None => self.run(&["set", "-uw", "-t", &target, "@workmux_status"]),
        };
    }

    fn right(&self) -> String {
        self.run(&["display", "-p", "-t", "main:", "#{T:status-right}"])
            .trim()
            .to_string()
    }
}

impl Drop for Tmux {
    fn drop(&mut self) {
        let _ = self.cmd(&["kill-server"]).output();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

const AGENTS_ONLY: &str = "[sections]\nleft = []\nright = [\"agents\"]\n";

#[test]
fn agents_indicator_tracks_workmux_status() {
    let Some(tmux) = Tmux::start("agents", AGENTS_ONLY) else {
        eprintln!("tmux not installed; skipping");
        return;
    };
    for name in ["alpha", "beta", "gamma"] {
        tmux.session(name);
    }

    assert_eq!(tmux.right(), "", "nothing pending");

    tmux.status("alpha", Some("🤖"));
    assert_eq!(tmux.right(), "", "working is not attention");

    tmux.status("beta", Some("💬"));
    assert_eq!(tmux.right(), "💬 beta", "one agent shows its worktree");

    tmux.status("gamma", Some("✅"));
    assert_eq!(tmux.right(), "💬 1 ✅ 1");

    tmux.status("alpha", Some("💬"));
    assert_eq!(tmux.right(), "💬 2 ✅ 1");

    tmux.status("gamma", None);
    tmux.status("alpha", None);
    assert_eq!(tmux.right(), "💬 beta", "cleared (focused) agents drop out");
}

#[test]
fn custom_icons_need_no_escaping() {
    // Commas, braces, tmux aliases (#S), strftime (%H), quotes and styles.
    let icon = "#[fg=#a6e3a1,bold]a,b}#S%H'q#[default]";
    let config = format!("{AGENTS_ONLY}[components.agents]\nwaiting = \"{icon}\"\n");
    let Some(tmux) = Tmux::start("icons", &config) else {
        eprintln!("tmux not installed; skipping");
        return;
    };
    tmux.session("alpha");
    tmux.session("beta");

    tmux.status("alpha", Some("💬"));
    assert_eq!(tmux.right(), "", "default icon no longer means waiting");

    tmux.status("alpha", Some(icon));
    assert_eq!(tmux.right(), format!("{icon} alpha"));

    tmux.status("beta", Some(icon));
    assert_eq!(tmux.right(), format!("{icon} 2"));
}
