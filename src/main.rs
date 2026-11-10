mod components;
mod config;
mod generate;
mod theme;

use std::io::Write;
use std::process::{Command, ExitCode, Stdio};

const USAGE: &str = "usage: wmux-line <apply|print>

  apply   load the status line into the running tmux server
  print   print the tmux commands apply would run";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["apply"] => snippet().and_then(|s| apply(&s)),
        ["print"] => snippet().map(|s| print!("{s}")),
        ["-h" | "--help" | "help"] => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("wmux-line: {e}");
            ExitCode::FAILURE
        }
    }
}

/// The config is fully loaded and validated before anything touches tmux.
fn snippet() -> Result<String, String> {
    let path = config::default_path()
        .ok_or("cannot locate config: neither XDG_CONFIG_HOME nor HOME is set")?;
    Ok(generate::generate(&config::load(&path)?))
}

/// One `tmux source-file -`, so every option lands in a single command. tmux
/// finds the server from `$TMUX`, as it does under `run-shell`.
fn apply(snippet: &str) -> Result<(), String> {
    let mut tmux = Command::new("tmux")
        .args(["source-file", "-"])
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot run tmux: {e}"))?;
    tmux.stdin
        .take()
        .expect("stdin is piped")
        .write_all(snippet.as_bytes())
        .map_err(|e| format!("cannot write to tmux: {e}"))?;
    let out = tmux.wait_with_output().map_err(|e| format!("tmux: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "tmux: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}
