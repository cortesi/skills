//! CLI parsing and command dispatch.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use crate::{commands, error::Result};

/// Parsed command line arguments.
#[derive(Debug, Parser)]
#[command(name = "skills", version, about = "Manage agent skills")]
struct Cli {
    /// Control colored output.
    #[arg(long, value_enum, default_value = "auto")]
    color: ColorMode,
    /// Enable verbose output.
    #[arg(long)]
    verbose: bool,
    /// Command to execute.
    #[command(subcommand)]
    command: Command,
}

/// Supported color output modes.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum ColorMode {
    /// Only colorize when stdout is a TTY.
    Auto,
    /// Always colorize output.
    Always,
    /// Never colorize output.
    Never,
}

/// Top-level subcommands.
#[derive(Debug, Subcommand)]
enum Command {
    /// List skills and their sync status.
    #[command(alias = "ls")]
    List,
    /// Push source skills to tools.
    Push {
        /// Limit pushes to a single skill.
        skill: Option<String>,
        /// Preview changes without writing.
        #[arg(long, short = 'n')]
        dry_run: bool,
        /// Overwrite modified skills without prompting.
        #[arg(long, short = 'f')]
        force: bool,
    },
    /// Pull tool skills back into sources.
    Pull {
        /// Limit pulls to a single skill.
        skill: Option<String>,
        /// Target source directory when multiple are configured.
        #[arg(long)]
        to: Option<PathBuf>,
    },
    /// Show diffs between sources and tool copies.
    Diff {
        /// Limit diffs to a single skill.
        skill: Option<String>,
        /// Send diff output through a pager.
        #[arg(long)]
        pager: Option<String>,
    },
    /// Initialize a skills config file.
    Init,
    /// Create a new skill template at a path.
    New {
        /// Destination directory for the new skill.
        path: PathBuf,
    },
}

/// Run the requested command.
pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    let color = cli.color.into_choice();

    match cli.command {
        Command::List => commands::list::run(color, cli.verbose).await,
        Command::Push {
            skill,
            dry_run,
            force,
        } => commands::push::run(color, cli.verbose, skill, dry_run, force).await,
        Command::Pull { skill, to } => commands::pull::run(color, cli.verbose, skill, to).await,
        Command::Diff { skill, pager } => {
            commands::diff::run(color, cli.verbose, skill, pager).await
        }
        Command::Init => commands::init::run().await,
        Command::New { path } => commands::new::run(path).await,
    }
}

impl ColorMode {
    /// Convert a CLI color mode into a color choice.
    fn into_choice(self) -> commands::ColorChoice {
        match self {
            Self::Auto => commands::ColorChoice::Auto,
            Self::Always => commands::ColorChoice::Always,
            Self::Never => commands::ColorChoice::Never,
        }
    }
}
