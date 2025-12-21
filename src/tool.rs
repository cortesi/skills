//! Tool-specific metadata and directory discovery.

use std::path::PathBuf;

use clap::ValueEnum;

use crate::error::{Error, Result};

/// Supported tool targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tool {
    /// Claude Code skills.
    Claude,
    /// OpenAI Codex skills.
    Codex,
}

/// Tool filter for CLI commands.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ToolFilter {
    /// Claude Code only.
    Claude,
    /// Codex only.
    Codex,
    /// All tools.
    All,
}

impl ToolFilter {
    /// Convert filter to list of tools.
    pub fn to_tools(self) -> Vec<Tool> {
        match self {
            Self::Claude => vec![Tool::Claude],
            Self::Codex => vec![Tool::Codex],
            Self::All => Tool::all().to_vec(),
        }
    }
}

impl Tool {
    /// Return all supported tools.
    pub(crate) fn all() -> [Self; 2] {
        [Self::Claude, Self::Codex]
    }

    /// Return the identifier used in templates.
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }

    /// Return the display name for user output.
    pub(crate) fn display_name(self) -> &'static str {
        match self {
            Self::Claude => "Claude Code",
            Self::Codex => "Codex",
        }
    }

    /// Return the global skills directory for this tool.
    pub(crate) fn skills_dir(self) -> Result<PathBuf> {
        let home = dirs::home_dir().ok_or(Error::HomeDirMissing)?;
        let dir = match self {
            Self::Claude => home.join(".claude").join("skills"),
            Self::Codex => home.join(".codex").join("skills"),
        };
        Ok(dir)
    }

    /// Return the local skills directory name for this tool (relative to project root).
    pub(crate) fn local_skills_dir(self) -> PathBuf {
        match self {
            Self::Claude => PathBuf::from(".claude").join("skills"),
            Self::Codex => PathBuf::from(".codex").join("skills"),
        }
    }
}
