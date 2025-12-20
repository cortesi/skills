//! Implementation of the `skills push` command.

use std::{
    fs,
    path::{Path, PathBuf},
};

use inquire::{Confirm, error::InquireError};

use crate::{
    catalog::Catalog,
    commands::{ColorChoice, init},
    config::Config,
    diagnostics::Diagnostics,
    error::{Error, Result},
    skill::{SKILL_FILE_NAME, SkillTemplate, render_template},
    status::normalize_line_endings,
    tool::Tool,
};

/// Execute the push command.
pub async fn run(
    _color: ColorChoice,
    verbose: bool,
    skill: String,
    claude: bool,
    codex: bool,
    dry_run: bool,
    force: bool,
) -> Result<()> {
    init::ensure().await?;
    let mut diagnostics = Diagnostics::new(verbose);
    let config = Config::load()?;
    let catalog = Catalog::load(&config, &mut diagnostics);

    let Some(template) = catalog.sources.get(&skill) else {
        return Err(Error::SkillNotFound { name: skill });
    };

    let tools = select_tools(claude, codex);

    println!("Pushing {skill}...");
    let results = push_skill(&catalog, template, &tools, dry_run, force, &mut diagnostics)?;
    for result in results {
        println!(
            "  {:<6}: {} ({})",
            result.tool_label, result.marker, result.summary
        );
    }

    diagnostics.print_skipped_summary();
    diagnostics.print_warning_summary();
    Ok(())
}

/// Select tools based on CLI flags.
fn select_tools(claude: bool, codex: bool) -> Vec<Tool> {
    if !claude && !codex {
        // Default: all tools
        Tool::all().to_vec()
    } else {
        let mut tools = Vec::new();
        if claude {
            tools.push(Tool::Claude);
        }
        if codex {
            tools.push(Tool::Codex);
        }
        tools
    }
}

/// Push a skill to specified tools.
fn push_skill(
    catalog: &Catalog,
    skill: &SkillTemplate,
    tools: &[Tool],
    dry_run: bool,
    force: bool,
    diagnostics: &mut Diagnostics,
) -> Result<Vec<PushLine>> {
    let mut results = Vec::new();

    for &tool in tools {
        let tool_dir = tool.skills_dir()?;
        let rendered = match render_template(&skill.contents, tool) {
            Ok(rendered) => rendered,
            Err(error) => {
                diagnostics.warn_skipped(&skill.skill_path, error);
                results.push(PushLine {
                    tool_label: tool.id().to_string(),
                    marker: '!',
                    summary: "skipped".to_string(),
                });
                continue;
            }
        };

        let tool_map = catalog.tools.get(&tool);
        let tool_skill = tool_map.and_then(|skills| skills.get(&skill.name));
        let existing = tool_skill.map(|installed| &installed.contents);
        let status = match existing {
            None => PushStatus::New,
            Some(contents) => {
                if normalize_line_endings(&rendered) == normalize_line_endings(contents) {
                    PushStatus::Unchanged
                } else {
                    PushStatus::Modified
                }
            }
        };

        let request = PushRequest {
            skill,
            tool,
            tool_dir: &tool_dir,
            rendered: &rendered,
            status,
        };
        let result = apply_push(&request, dry_run, force)?;

        results.push(PushLine {
            tool_label: tool.id().to_string(),
            marker: result.marker,
            summary: result.summary,
        });
    }

    Ok(results)
}

/// Request parameters for a push operation.
struct PushRequest<'a> {
    /// Source skill template to apply.
    skill: &'a SkillTemplate,
    /// Target tool.
    tool: Tool,
    /// Tool root directory.
    tool_dir: &'a PathBuf,
    /// Rendered template content.
    rendered: &'a str,
    /// Precomputed push status.
    status: PushStatus,
}

/// Push status for a tool skill.
#[derive(Debug, Clone, Copy)]
enum PushStatus {
    /// Skill does not exist in the tool.
    New,
    /// Tool skill is already in sync.
    Unchanged,
    /// Tool skill is modified.
    Modified,
}

/// Output line for push summary.
struct PushLine {
    /// Tool label.
    tool_label: String,
    /// Output marker.
    marker: char,
    /// Summary label.
    summary: String,
}

/// Result of applying a push request.
struct PushResult {
    /// Output marker.
    marker: char,
    /// Summary label.
    summary: String,
}

/// Apply push logic and write skill files if needed.
fn apply_push(request: &PushRequest<'_>, dry_run: bool, force: bool) -> Result<PushResult> {
    match request.status {
        PushStatus::Unchanged => Ok(PushResult {
            marker: '=',
            summary: "unchanged".to_string(),
        }),
        PushStatus::New => {
            if !dry_run {
                write_tool_skill(request.tool_dir, &request.skill.name, request.rendered)?;
            }
            Ok(PushResult {
                marker: '+',
                summary: "new".to_string(),
            })
        }
        PushStatus::Modified => {
            if !force && !dry_run {
                let prompt = format!(
                    "Overwrite modified skill '{}' in {}?",
                    request.skill.name,
                    request.tool.display_name()
                );
                let confirmed = confirm(&prompt)?;
                if !confirmed {
                    return Ok(PushResult {
                        marker: '!',
                        summary: "skipped".to_string(),
                    });
                }
            }

            if !dry_run {
                write_tool_skill(request.tool_dir, &request.skill.name, request.rendered)?;
            }

            Ok(PushResult {
                marker: '~',
                summary: "pushed".to_string(),
            })
        }
    }
}

/// Prompt for confirmation.
fn confirm(message: &str) -> Result<bool> {
    match Confirm::new(message).with_default(false).prompt() {
        Ok(value) => Ok(value),
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            Err(Error::PromptCanceled)
        }
        Err(error) => Err(Error::PromptFailed {
            message: error.to_string(),
        }),
    }
}

/// Write a rendered skill to the tool directory.
fn write_tool_skill(tool_dir: &Path, name: &str, rendered: &str) -> Result<()> {
    let skill_dir = tool_dir.join(name);
    fs::create_dir_all(&skill_dir).map_err(|error| Error::SkillWrite {
        path: skill_dir.clone(),
        source: error,
    })?;

    let skill_path = skill_dir.join(SKILL_FILE_NAME);
    fs::write(&skill_path, rendered).map_err(|error| Error::SkillWrite {
        path: skill_path,
        source: error,
    })?;

    Ok(())
}
