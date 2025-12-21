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

// Command modules are ordered alphabetically - maintain this order.
/// Diff command implementation.
pub mod diff;
/// Edit command implementation.
pub mod edit;
/// Import command implementation.
pub mod import;
/// Init command implementation.
pub mod init;
/// List command implementation.
pub mod list;
/// Mv command implementation.
pub mod mv;
/// New command implementation.
pub mod new;
/// Pack command implementation.
pub mod pack;
/// Promote command implementation.
pub mod promote;
/// Pull command implementation.
pub mod pull;
/// Push command implementation.
pub mod push;
/// Render command implementation.
pub mod render;
/// Show command implementation.
pub mod show;
/// Sync command implementation.
pub mod sync;
/// Unload command implementation.
pub mod unload;
/// Validate command implementation.
pub mod validate;
