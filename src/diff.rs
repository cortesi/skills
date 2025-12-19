//! Diff rendering and pager output helpers.

use std::{
    env,
    io::Write,
    process::{Command, Stdio},
};

use owo_colors::OwoColorize;
use similar::TextDiff;

use crate::error::{Error, Result};

/// Render a unified diff between two texts.
pub fn unified_diff(old_label: &str, new_label: &str, old: &str, new: &str) -> String {
    TextDiff::from_lines(old, new)
        .unified_diff()
        .context_radius(3)
        .header(old_label, new_label)
        .to_string()
}

/// Colorize a unified diff string when enabled.
pub fn colorize_diff(diff: &str, color: bool) -> String {
    if !color {
        return diff.to_string();
    }

    let mut output = String::new();
    for line in diff.lines() {
        let colored = if line.starts_with("+++") || line.starts_with("---") {
            line.bold().to_string()
        } else if line.starts_with("@@") {
            line.cyan().to_string()
        } else if line.starts_with('+') {
            line.green().to_string()
        } else if line.starts_with('-') {
            line.red().to_string()
        } else {
            line.to_string()
        };
        output.push_str(&colored);
        output.push('\n');
    }

    if !diff.ends_with('\n') && output.ends_with('\n') {
        output.pop();
    }

    output
}

/// Write output either directly to stdout or through a pager command.
pub fn write_output(output: &str, pager: Option<&str>) -> Result<()> {
    if output.is_empty() {
        return Ok(());
    }

    let Some(pager) = pager else {
        print!("{output}");
        return Ok(());
    };

    let mut parts = shell_words::split(pager).map_err(|error| Error::PagerParse {
        message: error.to_string(),
    })?;
    let program = parts.first().cloned().ok_or_else(|| Error::PagerParse {
        message: "pager command is empty".to_string(),
    })?;
    let args = parts.split_off(1);

    let mut child = Command::new(&program)
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| Error::PagerSpawn {
            pager: program.clone(),
            source: error,
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(output.as_bytes())
            .map_err(|error| Error::PagerWrite {
                pager: program.clone(),
                source: error,
            })?;
    }

    let status = child.wait().map_err(|error| Error::PagerSpawn {
        pager: program.clone(),
        source: error,
    })?;

    if !status.success() {
        return Err(Error::PagerStatus {
            pager: program,
            status,
        });
    }

    Ok(())
}

/// Resolve the pager command to use for diff output.
pub fn resolve_pager(override_pager: Option<&str>) -> Option<String> {
    if let Some(pager) = override_pager {
        return Some(pager.to_string());
    }

    env_pager("GIT_PAGER")
        .or_else(|| git_config_pager("pager.diff"))
        .or_else(|| git_config_pager("core.pager"))
        .or_else(|| env_pager("PAGER"))
}

/// Resolve a pager from an environment variable.
fn env_pager(key: &str) -> Option<String> {
    let value = env::var(key).ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Resolve a pager from git config.
fn git_config_pager(key: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["config", "--get", key])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout);
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
