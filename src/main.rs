mod components;
mod config;
mod generate;

use std::process::ExitCode;

const USAGE: &str = "usage: wmux-line print";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["print"] => print(),
        ["-h" | "--help" | "help"] => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn print() -> ExitCode {
    let Some(path) = config::default_path() else {
        eprintln!("wmux-line: cannot locate config: neither XDG_CONFIG_HOME nor HOME is set");
        return ExitCode::FAILURE;
    };
    match config::load(&path) {
        Ok(cfg) => {
            print!("{}", generate::generate(&cfg));
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("wmux-line: {e}");
            ExitCode::FAILURE
        }
    }
}
