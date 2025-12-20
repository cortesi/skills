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
    /// Command to execute (defaults to list).
    #[command(subcommand)]
    command: Option<Command>,
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
    /// Move a local skill to the global skills directory.
    Uplift {
        /// Name of the local skill to uplift.
        skill: String,
        /// Preview changes without moving.
        #[arg(long, short = 'n')]
        dry_run: bool,
        /// Overwrite existing global skill without prompting.
        #[arg(long, short = 'f')]
        force: bool,
        /// Specify tool when skill exists in both .claude and .codex.
        #[arg(long)]
        tool: Option<String>,
    },
    /// Package skills into ZIP files for sharing.
    Pack {
        /// Names of skills to pack.
        #[arg(required = true)]
        skills: Vec<String>,
        /// Output directory for ZIP files.
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
        /// Pack from local project skills instead of sources.
        #[arg(long)]
        local: bool,
        /// Preview what would be packed without creating files.
        #[arg(long, short = 'n')]
        dry_run: bool,
        /// Overwrite existing ZIP files.
        #[arg(long, short = 'f')]
        force: bool,
    },
    /// Package all skills into ZIP files.
    PackAll {
        /// Output directory for ZIP files.
        output: PathBuf,
        /// Pack from local project skills instead of sources.
        #[arg(long)]
        local: bool,
        /// Preview what would be packed without creating files.
        #[arg(long, short = 'n')]
        dry_run: bool,
        /// Overwrite existing ZIP files.
        #[arg(long, short = 'f')]
        force: bool,
    },
    /// Import a skill from a ZIP file, URL, or GitHub.
    Import {
        /// Path to ZIP file, URL, or GitHub URL.
        source: String,
        /// Import to specific location: claude, codex, source, or path.
        #[arg(long)]
        to: Option<String>,
        /// Import as local project skill.
        #[arg(long)]
        local: bool,
        /// Overwrite existing skill without prompting.
        #[arg(long, short = 'f')]
        force: bool,
        /// Preview what would be imported without extracting.
        #[arg(long, short = 'n')]
        dry_run: bool,
    },
}

/// Run the requested command.
pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    let color = cli.color.into_choice();

    match cli.command.unwrap_or(Command::List) {
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
        Command::Uplift {
            skill,
            dry_run,
            force,
            tool,
        } => commands::uplift::run(color, cli.verbose, skill, dry_run, force, tool).await,
        Command::Pack {
            skills,
            output,
            local,
            dry_run,
            force,
        } => commands::pack::run(color, cli.verbose, skills, output, local, dry_run, force).await,
        Command::PackAll {
            output,
            local,
            dry_run,
            force,
        } => commands::pack::run_all(color, cli.verbose, output, local, dry_run, force).await,
        Command::Import {
            source,
            to,
            local,
            force,
            dry_run,
        } => commands::import::run(color, cli.verbose, source, to, local, force, dry_run).await,
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
