#![warn(missing_docs)]
//! Library support for the skills CLI.

/// Catalog loading for source and tool skills.
mod catalog;
/// Command-line interface wiring and dispatch.
mod cli;
/// Command implementations.
mod commands;
/// Configuration loading and validation.
mod config;
/// Common diagnostics and warning aggregation.
mod diagnostics;
/// Unified diff rendering helpers.
mod diff;
/// Error handling for the crate.
mod error;
/// YAML frontmatter parsing for skills.
mod frontmatter;
/// Color palette and styling for CLI output.
mod palette;
/// Path expansion and normalization utilities.
mod paths;
/// Skill loading and templating helpers.
mod skill;
/// Status computation for list/diff operations.
mod status;
/// Tool directory discovery and metadata.
mod tool;

pub use crate::error::{Error, Result};

/// Run the CLI, returning a structured error on failure.
pub async fn run() -> Result<()> {
    cli::run().await
}
