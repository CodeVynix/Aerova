//! Aerova xtask: build/run/test wrappers (mirrors Lumora xtask style).

use std::process::{Command, ExitCode};

fn run(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn main() -> ExitCode {
    let what = std::env::args().nth(1);
    match what.as_deref() {
        Some("build") => {
            if run("cargo", &["build", "--workspace"]) {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Some("test") => {
            if run("cargo", &["test", "--workspace"]) {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Some("lint") => {
            if run(
                "cargo",
                &[
                    "clippy",
                    "--workspace",
                    "--all-targets",
                    "--",
                    "-D",
                    "warnings",
                ],
            ) {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Some("fmt") => {
            if run("cargo", &["fmt", "--check"]) {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        _ => {
            eprintln!("usage: cargo xtask <build|test|lint|fmt>");
            ExitCode::from(2)
        }
    }
}
