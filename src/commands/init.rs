//! Implementation of the `skills init` command.

use std::{
    fs,
    path::{Path, PathBuf},
};

use inquire::{Confirm, Text, error::InquireError};
use serde::Serialize;

use crate::{
    error::{Error, Result},
    paths::{default_config_path, display_path, expand_source_path},
};

/// Execute the init command.
pub async fn run() -> Result<()> {
    init(InitMode::Explicit).await
}

/// Ensure a config exists, running init if needed.
pub async fn ensure() -> Result<()> {
    init(InitMode::Auto).await
}

/// Mode for init execution.
#[derive(Debug, Clone, Copy)]
enum InitMode {
    /// Run init because the user requested it.
    Explicit,
    /// Run init automatically when config is missing.
    Auto,
}

/// Serialized config payload for init.
#[derive(Debug, Serialize)]
struct InitConfig {
    /// Configured source directories.
    sources: Vec<String>,
}

/// Run init for the requested mode.
async fn init(mode: InitMode) -> Result<()> {
    let config_path = default_config_path()?;
    if config_path.is_file() {
        if matches!(mode, InitMode::Explicit) {
            println!("Config already exists at {}", display_path(&config_path));
        }
        return Ok(());
    }

    if matches!(mode, InitMode::Auto) {
        println!("No config found. Starting `skills init`...");
    }

    let config_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
    let default_source = default_source_dir()?;
    let default_label = display_path(&default_source);

    let response = Text::new("Skills source directory")
        .with_default(&default_label)
        .prompt();
    let response = match response {
        Ok(value) => value,
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            return Err(Error::PromptCanceled);
        }
        Err(error) => {
            return Err(Error::PromptFailed {
                message: error.to_string(),
            });
        }
    };

    let trimmed = response.trim();
    let raw = if trimmed.is_empty() {
        default_label.as_str()
    } else {
        trimmed
    };

    let expanded = expand_source_path(raw, config_dir)?;
    if !expanded.is_dir() {
        let prompt = format!("Create directory {}?", display_path(&expanded));
        let create = confirm(&prompt)?;
        if create {
            fs::create_dir_all(&expanded).map_err(|error| Error::ConfigWrite {
                path: expanded.clone(),
                source: error,
            })?;
        } else {
            return Err(Error::PathMissing { path: expanded });
        }
    }

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).map_err(|error| Error::ConfigWrite {
            path: parent.to_path_buf(),
            source: error,
        })?;
    }

    let config = InitConfig {
        sources: vec![expanded.to_string_lossy().to_string()],
    };
    let contents =
        toml::to_string(&config).map_err(|error| Error::ConfigSerialize { source: error })?;

    fs::write(&config_path, contents).map_err(|error| Error::ConfigWrite {
        path: config_path.clone(),
        source: error,
    })?;

    println!("Created config at {}", display_path(&config_path));
    Ok(())
}

/// Prompt for confirmation during init.
fn confirm(message: &str) -> Result<bool> {
    match Confirm::new(message).with_default(true).prompt() {
        Ok(value) => Ok(value),
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            Err(Error::PromptCanceled)
        }
        Err(error) => Err(Error::PromptFailed {
            message: error.to_string(),
        }),
    }
}

/// Build a default source directory suggestion.
fn default_source_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or(Error::HomeDirMissing)?;
    Ok(home.join("skills"))
}
