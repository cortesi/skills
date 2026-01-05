//! Implementation of the `skills push` command.

use std::{
    fs,
    path::{Path, PathBuf},
};

use inquire::{Confirm, error::InquireError};
use similar::{ChangeTag, TextDiff};

use crate::{
    catalog::Catalog,
    commands::{ColorChoice, init},
    config::Config,
    diagnostics::Diagnostics,
    error::{Error, Result},
    palette::{fmt_label, fmt_skill_name},
    skill::{SKILL_FILE_NAME, SkillTemplate, render_template},
    status::normalize_line_endings,
    tool::{Tool, ToolFilter},
};

/// Execute the push command.
#[allow(clippy::too_many_arguments)]
pub async fn run(
    color: ColorChoice,
    verbose: bool,
    skills: Vec<String>,
    all: bool,
    tool_filter: ToolFilter,
    dry_run: bool,
    force: bool,
    yes: bool,
) -> Result<()> {
    init::ensure().await?;
    let mut diagnostics = Diagnostics::new(verbose);
    let config = Config::load()?;
    let catalog = Catalog::load(&config, &mut diagnostics);
    let use_color = color.enabled();

    let tools = tool_filter.to_tools();

    // Determine which skills to push
    let skill_names: Vec<String> = if all {
        // Push all source skills
        catalog.sources.keys().cloned().collect()
    } else if skills.is_empty() {
        // No skills specified - find out-of-sync skills and confirm
        let out_of_sync = find_out_of_sync_skills(&catalog, &tools, &mut diagnostics);
        if out_of_sync.is_empty() {
            println!("All skills are in sync.");
            return Ok(());
        }
        if !force && !dry_run {
            println!("Skills needing push:");
            for name in &out_of_sync {
                println!("  {}", fmt_skill_name(name, use_color));
            }
            println!();
            let prompt = format!("Push {} skill(s)?", out_of_sync.len());
            if !confirm(&prompt)? {
                println!("Aborted.");
                return Ok(());
            }
            println!();
        }
        out_of_sync
    } else {
        // Validate that all specified skills exist
        for name in &skills {
            if !catalog.sources.contains_key(name) {
                return Err(Error::SkillNotFound { name: name.clone() });
            }
        }
        skills
    };

    if skill_names.is_empty() {
        println!("No skills to push.");
        return Ok(());
    }

    // Sort skills for consistent output
    let mut skill_names = skill_names;
    skill_names.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));

    let total = skill_names.len();
    let mut pushed_count = 0;
    let mut skipped_count = 0;

    for name in &skill_names {
        let template = catalog.sources.get(name).unwrap();
        let results = push_skill(&catalog, template, &tools, dry_run, force, yes, use_color, &mut diagnostics)?;

        // Check if any actual push happened
        let any_pushed = results.iter().any(|r| r.marker == '+' || r.marker == '~');
        let any_skipped = results.iter().any(|r| r.marker == '!');

        if any_pushed {
            pushed_count += 1;
        }
        if any_skipped {
            skipped_count += 1;
        }

        // Print results
        println!("{}", fmt_skill_name(name, use_color));
        for result in results {
            println!(
                "    {:<6}: {} ({})",
                result.tool_label, result.marker, result.summary
            );
        }
    }

    println!();
    if dry_run {
        println!(
            "{} {} skill(s) would be pushed.",
            fmt_label("Dry run:", use_color),
            total
        );
    } else {
        println!(
            "{} {} pushed, {} skipped.",
            fmt_label("Done:", use_color),
            pushed_count,
            skipped_count
        );
    }

    diagnostics.print_skipped_summary();
    diagnostics.print_warning_summary();
    Ok(())
}

/// Find skills that are out of sync (source differs from at least one tool).
fn find_out_of_sync_skills(
    catalog: &Catalog,
    tools: &[Tool],
    diagnostics: &mut Diagnostics,
) -> Vec<String> {
    let mut out_of_sync = Vec::new();

    for (name, source) in &catalog.sources {
        for &tool in tools {
            let tool_map = catalog.tools.get(&tool);
            let tool_skill = tool_map.and_then(|skills| skills.get(name));

            // Render the template for this tool
            let rendered = match render_template(&source.contents, tool) {
                Ok(rendered) => rendered,
                Err(error) => {
                    diagnostics.warn_skipped(&source.skill_path, error);
                    continue;
                }
            };

            match tool_skill {
                None => {
                    // Skill missing from tool
                    out_of_sync.push(name.clone());
                    break;
                }
                Some(installed) => {
                    if normalize_line_endings(&rendered)
                        != normalize_line_endings(&installed.contents)
                    {
                        // Skill differs
                        out_of_sync.push(name.clone());
                        break;
                    }
                }
            }
        }
    }

    out_of_sync.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    out_of_sync
}


/// Push a skill to specified tools.
#[allow(clippy::too_many_arguments)]
fn push_skill(
    catalog: &Catalog,
    skill: &SkillTemplate,
    tools: &[Tool],
    dry_run: bool,
    force: bool,
    yes: bool,
    use_color: bool,
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
        let existing = tool_skill.map(|installed| installed.contents.clone());
        let status = match &existing {
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
            existing: existing.as_deref(),
            status,
        };
        let result = apply_push(&request, dry_run, force, yes, use_color)?;

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
    /// Existing content in tool (if any).
    existing: Option<&'a str>,
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
fn apply_push(
    request: &PushRequest<'_>,
    dry_run: bool,
    force: bool,
    yes: bool,
    use_color: bool,
) -> Result<PushResult> {
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
            if !dry_run {
                // Always prompt unless --yes is specified
                let skip_prompt = force && yes;

                if !skip_prompt {
                    // Show diff if force is specified (so user sees what's changing)
                    if force {
                        if let Some(existing) = request.existing {
                            println!();
                            println!(
                                "Diff for '{}' in {}:",
                                request.skill.name,
                                request.tool.display_name()
                            );
                            print_diff(existing, request.rendered, use_color);
                        }
                    }

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

                write_tool_skill(request.tool_dir, &request.skill.name, request.rendered)?;
            }

            Ok(PushResult {
                marker: '~',
                summary: "pushed".to_string(),
            })
        }
    }
}

/// Print a unified diff between two strings.
fn print_diff(old: &str, new: &str, use_color: bool) {
    use owo_colors::OwoColorize;

    let diff = TextDiff::from_lines(old, new);

    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };

        if use_color {
            match change.tag() {
                ChangeTag::Delete => print!("{}", format!("{}{}", sign, change).red()),
                ChangeTag::Insert => print!("{}", format!("{}{}", sign, change).green()),
                ChangeTag::Equal => print!(" {}", change),
            }
        } else {
            print!("{}{}", sign, change);
        }
    }
    println!();
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

#[cfg(test)]
mod tests {
    use crate::{
        diagnostics::Diagnostics,
        testutil::{TestFixture, simple_skill, skill_content},
        tool::Tool,
    };

    use super::find_out_of_sync_skills;

    #[test]
    fn finds_missing_skill() {
        let fixture = TestFixture::new()
            .with_source_skill("new-skill", &simple_skill("new-skill"));

        let catalog = fixture.catalog();
        let mut diagnostics = Diagnostics::new(false);
        let out_of_sync = find_out_of_sync_skills(&catalog, &[Tool::Claude], &mut diagnostics);

        assert_eq!(out_of_sync, vec!["new-skill"]);
    }

    #[test]
    fn finds_modified_skill() {
        let source_content = skill_content("modified", "desc", "source version");
        let tool_content = skill_content("modified", "desc", "tool version");

        let fixture = TestFixture::new()
            .with_source_skill("modified", &source_content)
            .with_tool_skill(Tool::Claude, "modified", &tool_content);

        let catalog = fixture.catalog();
        let mut diagnostics = Diagnostics::new(false);
        let out_of_sync = find_out_of_sync_skills(&catalog, &[Tool::Claude], &mut diagnostics);

        assert_eq!(out_of_sync, vec!["modified"]);
    }

    #[test]
    fn ignores_synced_skill() {
        let content = simple_skill("synced");

        let fixture = TestFixture::new()
            .with_source_skill("synced", &content)
            .with_tool_skill(Tool::Claude, "synced", &content);

        let catalog = fixture.catalog();
        let mut diagnostics = Diagnostics::new(false);
        let out_of_sync = find_out_of_sync_skills(&catalog, &[Tool::Claude], &mut diagnostics);

        assert!(out_of_sync.is_empty());
    }

    #[test]
    fn checks_all_specified_tools() {
        let content = simple_skill("partial");

        // Synced with Claude but missing from Codex
        let fixture = TestFixture::new()
            .with_source_skill("partial", &content)
            .with_tool_skill(Tool::Claude, "partial", &content);

        let catalog = fixture.catalog();
        let mut diagnostics = Diagnostics::new(false);

        // Check only Claude - should be synced
        let out_of_sync = find_out_of_sync_skills(&catalog, &[Tool::Claude], &mut diagnostics);
        assert!(out_of_sync.is_empty());

        // Check only Codex - should be out of sync (missing)
        let out_of_sync = find_out_of_sync_skills(&catalog, &[Tool::Codex], &mut diagnostics);
        assert_eq!(out_of_sync, vec!["partial"]);

        // Check both - should be out of sync
        let out_of_sync =
            find_out_of_sync_skills(&catalog, &[Tool::Claude, Tool::Codex], &mut diagnostics);
        assert_eq!(out_of_sync, vec!["partial"]);
    }

    #[test]
    fn sorts_results_case_insensitively() {
        let fixture = TestFixture::new()
            .with_source_skill("Zebra", &simple_skill("Zebra"))
            .with_source_skill("apple", &simple_skill("apple"))
            .with_source_skill("Banana", &simple_skill("Banana"));

        let catalog = fixture.catalog();
        let mut diagnostics = Diagnostics::new(false);
        let out_of_sync = find_out_of_sync_skills(&catalog, &[Tool::Claude], &mut diagnostics);

        assert_eq!(out_of_sync, vec!["apple", "Banana", "Zebra"]);
    }

    #[test]
    fn ignores_orphan_tool_skills() {
        // Tool has a skill that source doesn't - should not appear in out_of_sync
        let fixture = TestFixture::new()
            .with_tool_skill(Tool::Claude, "orphan", &simple_skill("orphan"));

        let catalog = fixture.catalog();
        let mut diagnostics = Diagnostics::new(false);
        let out_of_sync = find_out_of_sync_skills(&catalog, &[Tool::Claude], &mut diagnostics);

        assert!(out_of_sync.is_empty());
    }
}
