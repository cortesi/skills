//! Implementation of the `skills pull` command.

use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use inquire::{Confirm, Text, error::InquireError};

use crate::{
    catalog::Catalog,
    commands::{ColorChoice, init},
    config::Config,
    diagnostics::Diagnostics,
    diff::{colorize_diff, unified_diff, write_output},
    error::{Error, Result},
    paths::{display_path, expand_source_path},
    skill::{SKILL_FILE_NAME, SkillTemplate, ToolSkill, render_template},
    status::normalize_line_endings,
    tool::Tool,
};

/// Execute the pull command.
pub async fn run(
    color: ColorChoice,
    verbose: bool,
    skill: Option<String>,
    to: Option<PathBuf>,
) -> Result<()> {
    init::ensure().await?;
    let mut diagnostics = Diagnostics::new(verbose);
    let config = Config::load()?;
    let catalog = Catalog::load(&config, &mut diagnostics);
    let target_override = resolve_target_override(&to)?;

    let mut plans = collect_pull_plans(&catalog, skill.as_deref(), &mut diagnostics)?;
    if plans.is_empty() {
        println!("No modified skills found.");
        return Ok(());
    }

    plans.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));

    print_plan_summary(&plans);

    let use_color = color.enabled();

    for plan in plans.drain(..) {
        match plan.variants.as_slice() {
            [] => continue,
            [variant] => {
                if !confirm_pull(&plan, variant)? {
                    continue;
                }
                let target = select_target_source(&config, &plan, target_override.as_ref())?;
                apply_pull(&plan, variant, &target)?;
                println!(
                    "Pulled {} from {} -> {}",
                    plan.name,
                    variant.tool.display_name(),
                    display_path(&target)
                );
            }
            variants => {
                let selected = resolve_conflict(&plan, variants, use_color)?;
                let Some(selected) = selected else {
                    continue;
                };
                let target = select_target_source(&config, &plan, target_override.as_ref())?;
                apply_pull(&plan, &selected, &target)?;
                println!(
                    "Pulled {} from {} -> {}",
                    plan.name,
                    selected.tool.display_name(),
                    display_path(&target)
                );
            }
        }
    }

    diagnostics.print_skipped_summary();
    diagnostics.print_warning_summary();
    Ok(())
}

/// Pull plan for a single skill.
#[derive(Debug, Clone)]
struct PullPlan {
    /// Skill name.
    name: String,
    /// Source skill if present.
    source: Option<SkillTemplate>,
    /// Tool variants to pull from.
    variants: Vec<PullVariant>,
}

/// Tool-specific variant for pulling.
#[derive(Debug, Clone)]
struct PullVariant {
    /// Tool origin for the variant.
    tool: Tool,
    /// Tool skill contents.
    skill: ToolSkill,
    /// Whether this is an orphaned skill.
    orphan: bool,
}

/// Build pull plans from catalog data.
fn collect_pull_plans(
    catalog: &Catalog,
    skill: Option<&str>,
    diagnostics: &mut Diagnostics,
) -> Result<Vec<PullPlan>> {
    let mut names = Vec::new();
    for name in catalog.sources.keys() {
        names.push(name.clone());
    }
    for tool_map in catalog.tools.values() {
        for name in tool_map.keys() {
            if !names.contains(name) {
                names.push(name.clone());
            }
        }
    }

    if let Some(skill) = skill {
        if !names.iter().any(|name| name == skill) {
            return Err(Error::SkillNotFound {
                name: skill.to_string(),
            });
        }
        names.retain(|name| name == skill);
    }

    let mut plans = Vec::new();
    for name in names {
        let source = catalog.sources.get(&name).cloned();
        let mut variants = Vec::new();

        for tool in Tool::all() {
            let tool_map = catalog.tools.get(&tool);
            let tool_skill = tool_map.and_then(|skills| skills.get(&name));
            let Some(tool_skill) = tool_skill else {
                continue;
            };

            if let Some(source) = &source {
                let rendered = match render_template(&source.contents, tool) {
                    Ok(rendered) => rendered,
                    Err(error) => {
                        diagnostics.warn_skipped(&source.skill_path, error);
                        variants.clear();
                        break;
                    }
                };

                let modified = normalize_line_endings(&rendered)
                    != normalize_line_endings(&tool_skill.contents);
                if modified {
                    variants.push(PullVariant {
                        tool,
                        skill: tool_skill.clone(),
                        orphan: false,
                    });
                }
            } else {
                variants.push(PullVariant {
                    tool,
                    skill: tool_skill.clone(),
                    orphan: true,
                });
            }
        }

        if !variants.is_empty() {
            plans.push(PullPlan {
                name,
                source,
                variants,
            });
        }
    }

    Ok(plans)
}

/// Print a summary of pull candidates.
fn print_plan_summary(plans: &[PullPlan]) {
    println!("Found {} modified skills:\n", plans.len());
    for plan in plans {
        let source_path = plan
            .source
            .as_ref()
            .map(|skill| display_path(&skill.skill_path))
            .unwrap_or_else(|| "-".to_string());
        println!("{}", plan.name);
        println!("  source: {}", source_path);
        for variant in &plan.variants {
            println!(
                "  tool:   {} ({})",
                display_path(&variant.skill.skill_path),
                variant.tool.display_name()
            );
        }
        println!();
    }
}

/// Confirm pulling a single variant.
fn confirm_pull(plan: &PullPlan, variant: &PullVariant) -> Result<bool> {
    let prompt = if variant.orphan {
        format!(
            "Create skill '{}' from {}?",
            plan.name,
            variant.tool.display_name()
        )
    } else {
        format!(
            "Pull changes for '{}' from {}?",
            plan.name,
            variant.tool.display_name()
        )
    };
    confirm(&prompt)
}

/// Resolve conflicts between multiple tool variants.
fn resolve_conflict(
    plan: &PullPlan,
    variants: &[PullVariant],
    color: bool,
) -> Result<Option<PullVariant>> {
    loop {
        println!(
            "{} has different modifications in multiple tools:\n",
            plan.name
        );
        for (index, variant) in variants.iter().enumerate() {
            println!(
                "  [{}] {}  (modified {})",
                index + 1,
                variant.tool.display_name(),
                format_age(variant.skill.modified)
            );
        }
        println!("  [d] Show diff between versions");
        println!("  [s] Skip");

        let choice = Text::new("Which version to pull? [1/2/d/s]")
            .with_default("s")
            .prompt();

        let choice = match choice {
            Ok(value) => value,
            Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
                return Err(Error::PromptCanceled);
            }
            Err(error) => {
                return Err(Error::PromptFailed {
                    message: error.to_string(),
                });
            }
        };

        match choice.as_str() {
            "s" | "S" => return Ok(None),
            "d" | "D" => {
                show_variant_diff(variants, color)?;
                continue;
            }
            _ => {
                if let Ok(index) = choice.parse::<usize>()
                    && let Some(variant) = variants.get(index.saturating_sub(1))
                {
                    return Ok(Some(variant.clone()));
                }
            }
        }
    }
}

/// Show a diff between the first two variants.
fn show_variant_diff(variants: &[PullVariant], color: bool) -> Result<()> {
    if variants.len() < 2 {
        return Ok(());
    }

    let left = &variants[0];
    let right = &variants[1];
    let diff_text = unified_diff(
        &display_path(&left.skill.skill_path),
        &display_path(&right.skill.skill_path),
        &left.skill.contents,
        &right.skill.contents,
    );
    let diff_text = colorize_diff(&diff_text, color);
    write_output(&diff_text, None)?;
    Ok(())
}

/// Select the target source directory for a pull.
fn select_target_source(
    config: &Config,
    plan: &PullPlan,
    override_path: Option<&PathBuf>,
) -> Result<PathBuf> {
    if let Some(source) = &plan.source {
        return Ok(source.source_root.clone());
    }

    if let Some(path) = override_path {
        return Ok(path.clone());
    }

    let sources = config.sources();
    if sources.len() == 1 {
        return Ok(sources[0].clone());
    }

    println!("Available sources:");
    for (index, source) in sources.iter().enumerate() {
        println!("  [{}] {}", index + 1, display_path(source));
    }

    let prompt = "Select target source (press Enter for default)";
    let response = Text::new(prompt).with_default("1").prompt();
    let response = match response {
        Ok(value) => value,
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            return Err(Error::PromptCanceled);
        }
        Err(error) => {
            return Err(Error::PromptFailed {
                message: error.to_string(),
            });
        }
    };

    let trimmed = response.trim();
    if trimmed.is_empty() || trimmed == "1" {
        return Ok(sources[0].clone());
    }

    if let Ok(index) = trimmed.parse::<usize>()
        && let Some(source) = sources.get(index.saturating_sub(1))
    {
        return Ok(source.clone());
    }

    let normalized = expand_source_path(trimmed, Path::new("."))?;
    Ok(normalized)
}

/// Apply a pull variant to the selected source.
fn apply_pull(plan: &PullPlan, variant: &PullVariant, target: &Path) -> Result<()> {
    let skill_dir = if let Some(source) = &plan.source {
        source.skill_dir.clone()
    } else {
        target.join(&plan.name)
    };

    fs::create_dir_all(&skill_dir).map_err(|error| Error::SkillWrite {
        path: skill_dir.clone(),
        source: error,
    })?;

    let skill_path = skill_dir.join(SKILL_FILE_NAME);
    fs::write(&skill_path, &variant.skill.contents).map_err(|error| Error::SkillWrite {
        path: skill_path,
        source: error,
    })?;

    Ok(())
}

/// Resolve the `--to` override path if provided.
fn resolve_target_override(to: &Option<PathBuf>) -> Result<Option<PathBuf>> {
    let Some(path) = to else {
        return Ok(None);
    };

    let normalized = expand_source_path(
        path.to_str()
            .ok_or_else(|| Error::PathNotUnicode { path: path.clone() })?,
        Path::new("."),
    )?;

    if !normalized.is_dir() {
        return Err(Error::PathMissing { path: normalized });
    }

    Ok(Some(normalized))
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

/// Format a relative age from a timestamp.
fn format_age(modified: SystemTime) -> String {
    let elapsed = modified.elapsed().unwrap_or(Duration::from_secs(0));
    let seconds = elapsed.as_secs();
    if seconds < 60 {
        return "moments ago".to_string();
    }
    if seconds < 60 * 60 {
        let minutes = seconds / 60;
        return format!("{} minute{} ago", minutes, plural(minutes));
    }
    if seconds < 60 * 60 * 24 {
        let hours = seconds / 60 / 60;
        return format!("{} hour{} ago", hours, plural(hours));
    }
    let days = seconds / 60 / 60 / 24;
    format!("{days} day{} ago", plural(days))
}

/// Return a plural suffix for counts.
fn plural(count: u64) -> &'static str {
    if count == 1 { "" } else { "s" }
}
