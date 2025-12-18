//! Developer workflow tasks for the skills workspace.

use std::{
    env,
    path::Path,
    process::{Command, ExitCode, Stdio},
};

fn main() -> ExitCode {
    match parse_command() {
        Some(Task::Tidy) => run_tidy(),
        None => {
            eprintln!("Usage: cargo xtask tidy");
            ExitCode::from(2)
        }
    }
}

enum Task {
    Tidy,
}

fn parse_command() -> Option<Task> {
    let mut args = env::args();
    let _ = args.next();
    match args.next().as_deref() {
        Some("tidy") if args.next().is_none() => Some(Task::Tidy),
        _ => None,
    }
}

fn run_tidy() -> ExitCode {
    if !run_fmt() {
        return ExitCode::from(1);
    }

    if !run_clippy() {
        return ExitCode::from(1);
    }

    ExitCode::SUCCESS
}

fn run_fmt() -> bool {
    if Path::new("rustfmt-nightly.toml").exists() {
        run_command(
            "cargo",
            &[
                "+nightly",
                "fmt",
                "--all",
                "--",
                "--config-path",
                "./rustfmt-nightly.toml",
            ],
        )
    } else {
        run_command("cargo", &["+nightly", "fmt", "--all"])
    }
}

fn run_clippy() -> bool {
    run_command(
        "cargo",
        &[
            "clippy",
            "-q",
            "--fix",
            "--all",
            "--all-targets",
            "--all-features",
            "--allow-dirty",
            "--tests",
            "--examples",
        ],
    )
}

fn run_command(program: &str, args: &[&str]) -> bool {
    match Command::new(program)
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
    {
        Ok(status) if status.success() => true,
        Ok(status) => {
            eprintln!("Command `{program}` failed with status {status}");
            false
        }
        Err(err) => {
            eprintln!("Failed to run `{program}`: {err}");
            false
        }
    }
}
