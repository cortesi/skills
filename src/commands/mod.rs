//! CLI command implementations.

use std::io::{self, IsTerminal};

/// Output color handling selection.
#[derive(Debug, Clone, Copy)]
pub enum ColorChoice {
    /// Colorize only when output is a TTY.
    Auto,
    /// Always colorize output.
    Always,
    /// Never colorize output.
    Never,
}

impl ColorChoice {
    /// Determine whether color output should be enabled.
    pub(crate) fn enabled(self) -> bool {
        match self {
            Self::Auto => io::stdout().is_terminal(),
            Self::Always => true,
            Self::Never => false,
        }
    }
}

/// Diff command implementation.
pub mod diff;
/// Init command implementation.
pub mod init;
/// List command implementation.
pub mod list;
/// New command implementation.
pub mod new;
/// Pull command implementation.
pub mod pull;
/// Push command implementation.
pub mod push;
/// Uplift command implementation.
pub mod uplift;
