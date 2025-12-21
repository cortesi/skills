//! CLI parsing and command dispatch.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use crate::{commands, error::Result, tool::ToolFilter};

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

// Commands are ordered alphabetically - maintain this order.
/// Top-level subcommands.
#[derive(Debug, Subcommand)]
enum Command {
    /// Show diffs between sources and tool copies.
    Diff {
        /// Limit diffs to a single skill.
        skill: Option<String>,
        /// Send diff output through a pager.
        #[arg(long)]
        pager: Option<String>,
    },
    /// Open a skill in your editor.
    Edit {
        /// Name of the skill to edit.
        skill: String,
    },
    /// Import a skill from a ZIP file, URL, or GitHub.
    Import {
        /// Path to ZIP file, URL, or GitHub URL.
        source: String,
        /// Import to specific location: claude, codex, source, or path.
        #[arg(long)]
        to: Option<String>,
        /// Import as project-local skill (.claude/skills/, .codex/skills/).
        #[arg(long, alias = "local")]
        project: bool,
        /// Overwrite existing skill without prompting.
        #[arg(long, short = 'f')]
        force: bool,
        /// Preview what would be imported without extracting.
        #[arg(long, short = 'n')]
        dry_run: bool,
    },
    /// Initialize a skills config file.
    Init,
    /// List skills and their sync status.
    #[command(alias = "ls", alias = "status")]
    List,
    /// Rename a skill across source and tools.
    Mv {
        /// Current name of the skill.
        old_name: String,
        /// New name for the skill.
        new_name: String,
        /// Preview changes without renaming.
        #[arg(long, short = 'n')]
        dry_run: bool,
        /// Overwrite existing skill without prompting.
        #[arg(long, short = 'f')]
        force: bool,
    },
    /// Create a new skill template at a path.
    New {
        /// Destination directory for the new skill.
        path: PathBuf,
    },
    /// Package skills into ZIP files for sharing.
    Pack {
        /// Names of skills to pack (omit for all skills).
        skills: Vec<String>,
        /// Pack all skills.
        #[arg(long)]
        all: bool,
        /// Output directory for ZIP files.
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
        /// Pack from project-local skills instead of sources.
        #[arg(long, alias = "local")]
        project: bool,
        /// Preview what would be packed without creating files.
        #[arg(long, short = 'n')]
        dry_run: bool,
        /// Overwrite existing ZIP files.
        #[arg(long, short = 'f')]
        force: bool,
    },
    /// Package all skills into ZIP files (deprecated: use `pack --all -o <dir>`).
    #[command(hide = true)]
    PackAll {
        /// Output directory for ZIP files.
        output: PathBuf,
        /// Pack from project-local skills instead of sources.
        #[arg(long, alias = "local")]
        project: bool,
        /// Preview what would be packed without creating files.
        #[arg(long, short = 'n')]
        dry_run: bool,
        /// Overwrite existing ZIP files.
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
    /// Push source skills to tools.
    Push {
        /// Names of skills to push (omit for all out-of-sync skills).
        skills: Vec<String>,
        /// Push all skills.
        #[arg(long)]
        all: bool,
        /// Target tool (claude, codex, or all).
        #[arg(long, value_enum, default_value = "all")]
        tool: ToolFilter,
        /// Preview changes without writing.
        #[arg(long, short = 'n')]
        dry_run: bool,
        /// Overwrite modified skills (shows diff before prompting).
        #[arg(long, short = 'f')]
        force: bool,
        /// Skip all prompts (requires --force).
        #[arg(long, short = 'y', requires = "force")]
        yes: bool,
    },
    /// Preview rendered skill output for a specific tool.
    Render {
        /// Name of the skill to render.
        skill: String,
        /// Target tool to render for.
        #[arg(long, value_enum)]
        tool: ToolFilter,
    },
    /// Display a skill file with syntax highlighting.
    Show {
        /// Name of the skill to display.
        skill: String,
        /// Send output through a pager.
        #[arg(long)]
        pager: Option<String>,
    },
    /// Sync skills between sources and tools based on timestamps.
    Sync {
        /// Names of skills to sync (omit for all).
        skills: Vec<String>,
        /// On conflict, prefer source version.
        #[arg(long, conflicts_with = "prefer_tool")]
        prefer_source: bool,
        /// On conflict, prefer tool version (uses newest tool).
        #[arg(long, conflicts_with = "prefer_source")]
        prefer_tool: bool,
        /// Preview changes without writing.
        #[arg(long, short = 'n')]
        dry_run: bool,
    },
    /// Remove a skill from tool directories.
    Unload {
        /// Name of the skill to unload.
        skill: String,
        /// Target tool (claude, codex, or all).
        #[arg(long, value_enum, default_value = "all")]
        tool: ToolFilter,
        /// Preview changes without removing.
        #[arg(long, short = 'n')]
        dry_run: bool,
        /// Remove without prompting.
        #[arg(long, short = 'f')]
        force: bool,
    },
    /// Move a local skill to the global skills directory.
    #[command(alias = "uplift")]
    Promote {
        /// Name of the local skill to promote.
        skill: String,
        /// Specify tool when skill exists in both .claude and .codex (claude or codex).
        #[arg(long, value_enum)]
        tool: Option<ToolFilter>,
        /// Preview changes without moving.
        #[arg(long, short = 'n')]
        dry_run: bool,
        /// Overwrite existing global skill without prompting.
        #[arg(long, short = 'f')]
        force: bool,
    },
    /// Validate skill files for correct structure and syntax.
    Validate {
        /// Name of skill to validate (omit for all skills).
        skill: Option<String>,
    },
}

/// Run the requested command.
pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    let color = cli.color.into_choice();

    // Match arms are ordered alphabetically - maintain this order.
    match cli.command.unwrap_or(Command::List) {
        Command::Diff { skill, pager } => {
            commands::diff::run(color, cli.verbose, skill, pager).await
        }
        Command::Edit { skill } => commands::edit::run(cli.verbose, skill).await,
        Command::Import {
            source,
            to,
            project,
            force,
            dry_run,
        } => commands::import::run(color, cli.verbose, source, to, project, force, dry_run).await,
        Command::Init => commands::init::run().await,
        Command::List => commands::list::run(color, cli.verbose).await,
        Command::Mv {
            old_name,
            new_name,
            dry_run,
            force,
        } => commands::mv::run(color, cli.verbose, old_name, new_name, dry_run, force).await,
        Command::New { path } => commands::new::run(path).await,
        Command::Pack {
            skills,
            all,
            output,
            project,
            dry_run,
            force,
        } => commands::pack::run(color, cli.verbose, skills, all, output, project, dry_run, force).await,
        Command::PackAll {
            output,
            project,
            dry_run,
            force,
        } => commands::pack::run_all(color, cli.verbose, output, project, dry_run, force).await,
        Command::Pull { skill, to } => commands::pull::run(color, cli.verbose, skill, to).await,
        Command::Push {
            skills,
            all,
            tool,
            dry_run,
            force,
            yes,
        } => commands::push::run(color, cli.verbose, skills, all, tool, dry_run, force, yes).await,
        Command::Render { skill, tool } => {
            commands::render::run(color, cli.verbose, skill, tool).await
        }
        Command::Show { skill, pager } => {
            commands::show::run(color, cli.verbose, skill, pager).await
        }
        Command::Sync {
            skills,
            prefer_source,
            prefer_tool,
            dry_run,
        } => commands::sync::run(color, cli.verbose, skills, prefer_source, prefer_tool, dry_run).await,
        Command::Unload {
            skill,
            tool,
            dry_run,
            force,
        } => commands::unload::run(color, cli.verbose, skill, tool, dry_run, force).await,
        Command::Promote {
            skill,
            tool,
            dry_run,
            force,
        } => commands::promote::run(color, cli.verbose, skill, tool, dry_run, force).await,
        Command::Validate { skill } => commands::validate::run(color, cli.verbose, skill).await,
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
