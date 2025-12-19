//! Warning aggregation and diagnostic summaries.

use std::path::{Path, PathBuf};

/// Details about a skipped skill file.
#[derive(Debug, Clone)]
pub struct SkippedSkill {
    /// Path to the skipped skill file.
    pub(crate) path: PathBuf,
    /// Reason the skill was skipped.
    pub(crate) reason: String,
}

/// Aggregates warnings and skipped skills for a command run.
#[derive(Debug, Default)]
pub struct Diagnostics {
    /// Collected warning messages.
    warnings: Vec<String>,
    /// Collected skipped skill records.
    skipped: Vec<SkippedSkill>,
}

impl Diagnostics {
    /// Create a new diagnostics collector.
    pub(crate) fn new(_verbose: bool) -> Self {
        Self {
            warnings: Vec::new(),
            skipped: Vec::new(),
        }
    }

    /// Record a warning and print it immediately.
    pub(crate) fn warn(&mut self, message: impl Into<String>) {
        let message = message.into();
        eprintln!("Warning: {message}");
        self.warnings.push(message);
    }

    /// Print a non-warning continuation line.
    pub(crate) fn note(&self, message: impl Into<String>) {
        eprintln!("{}", message.into());
    }

    /// Record a skipped skill and emit the warning.
    pub(crate) fn warn_skipped(&mut self, path: &Path, reason: impl Into<String>) {
        let reason = reason.into();
        self.warn(format!("{} - {reason}", path.display()));
        self.skipped.push(SkippedSkill {
            path: path.to_path_buf(),
            reason,
        });
    }

    /// Print a summary for skipped skills if any were recorded.
    pub(crate) fn print_skipped_summary(&self) {
        if self.skipped.is_empty() {
            return;
        }

        eprintln!("Skipped {} skills due to errors:", self.skipped.len());
        for skipped in &self.skipped {
            eprintln!("  - {}: {}", skipped.path.display(), skipped.reason);
        }
    }

    /// Print a warning summary when warnings were emitted.
    pub(crate) fn print_warning_summary(&self) {
        if self.warnings.is_empty() {
            return;
        }

        eprintln!("Completed with {} warning(s).", self.warnings.len());
    }
}
