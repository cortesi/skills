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

/// Source of a tool skill variant (global or local).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VariantSource {
    /// Global tool directory (~/.claude/skills or ~/.codex/skills).
    Global,
    /// Local project directory (.claude/skills or .codex/skills).
    Local,
}

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
                    format_variant_source(variant),
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
                    format_variant_source(&selected),
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
    /// Source of the variant (global or local).
    source: VariantSource,
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
    // Include local skills in the name collection
    for local_map in catalog.local.values() {
        for name in local_map.keys() {
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
            // Check global tool skills
            let tool_map = catalog.tools.get(&tool);
            let tool_skill = tool_map.and_then(|skills| skills.get(&name));

            if let Some(tool_skill) = tool_skill {
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
                            source: VariantSource::Global,
                        });
                    }
                } else {
                    variants.push(PullVariant {
                        tool,
                        skill: tool_skill.clone(),
                        orphan: true,
                        source: VariantSource::Global,
                    });
                }
            }

            // Check local skills
            let local_map = catalog.local.get(&tool);
            let local_skill = local_map.and_then(|skills| skills.get(&name));

            if let Some(local_skill) = local_skill {
                // Convert LocalSkill to ToolSkill for variant
                let tool_skill = ToolSkill {
                    name: local_skill.name.clone(),
                    skill_path: local_skill.skill_path.clone(),
                    contents: local_skill.contents.clone(),
                    modified: local_skill.modified,
                };

                if let Some(source) = &source {
                    let rendered = match render_template(&source.contents, tool) {
                        Ok(rendered) => rendered,
                        Err(error) => {
                            diagnostics.warn_skipped(&source.skill_path, error);
                            continue;
                        }
                    };

                    let modified = normalize_line_endings(&rendered)
                        != normalize_line_endings(&local_skill.contents);
                    if modified {
                        variants.push(PullVariant {
                            tool,
                            skill: tool_skill,
                            orphan: false,
                            source: VariantSource::Local,
                        });
                    }
                } else {
                    variants.push(PullVariant {
                        tool,
                        skill: tool_skill,
                        orphan: true,
                        source: VariantSource::Local,
                    });
                }
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
            let source_label = match variant.source {
                VariantSource::Global => "",
                VariantSource::Local => " local",
            };
            println!(
                "  tool:   {} ({}{})",
                display_path(&variant.skill.skill_path),
                variant.tool.display_name(),
                source_label
            );
        }
        println!();
    }
}

/// Confirm pulling a single variant.
fn confirm_pull(plan: &PullPlan, variant: &PullVariant) -> Result<bool> {
    let source_label = match variant.source {
        VariantSource::Global => variant.tool.display_name().to_string(),
        VariantSource::Local => format!("{} local", variant.tool.display_name()),
    };
    let prompt = if variant.orphan {
        format!("Create skill '{}' from {}?", plan.name, source_label)
    } else {
        format!("Pull changes for '{}' from {}?", plan.name, source_label)
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
            "{} has different modifications in multiple locations:\n",
            plan.name
        );
        for (index, variant) in variants.iter().enumerate() {
            let source_label = match variant.source {
                VariantSource::Global => variant.tool.display_name().to_string(),
                VariantSource::Local => format!("{} local", variant.tool.display_name()),
            };
            println!(
                "  [{}] {}  (modified {})",
                index + 1,
                source_label,
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

/// Format the source label for a variant (e.g., "Claude Code" or "Claude Code local").
fn format_variant_source(variant: &PullVariant) -> String {
    match variant.source {
        VariantSource::Global => variant.tool.display_name().to_string(),
        VariantSource::Local => format!("{} local", variant.tool.display_name()),
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        diagnostics::Diagnostics,
        testutil::{TestFixture, simple_skill, skill_content},
        tool::Tool,
    };

    use super::{VariantSource, collect_pull_plans};

    #[test]
    fn detects_modified_global_tool_skill() {
        let source_content = skill_content("my-skill", "desc", "original");
        let tool_content = skill_content("my-skill", "desc", "modified");

        let fixture = TestFixture::new()
            .with_source_skill("my-skill", &source_content)
            .with_tool_skill(Tool::Claude, "my-skill", &tool_content);

        let catalog = fixture.catalog();
        let mut diagnostics = Diagnostics::new(false);
        let plans = collect_pull_plans(&catalog, None, &mut diagnostics).unwrap();

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].name, "my-skill");
        assert_eq!(plans[0].variants.len(), 1);
        assert_eq!(plans[0].variants[0].tool, Tool::Claude);
        assert_eq!(plans[0].variants[0].source, VariantSource::Global);
        assert!(!plans[0].variants[0].orphan);
    }

    #[test]
    fn detects_orphaned_global_tool_skill() {
        let fixture = TestFixture::new()
            .with_tool_skill(Tool::Codex, "orphan-skill", &simple_skill("orphan-skill"));

        let catalog = fixture.catalog();
        let mut diagnostics = Diagnostics::new(false);
        let plans = collect_pull_plans(&catalog, None, &mut diagnostics).unwrap();

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].name, "orphan-skill");
        assert_eq!(plans[0].variants.len(), 1);
        assert_eq!(plans[0].variants[0].tool, Tool::Codex);
        assert_eq!(plans[0].variants[0].source, VariantSource::Global);
        assert!(plans[0].variants[0].orphan);
    }

    #[test]
    fn detects_modified_local_skill() {
        let source_content = skill_content("local-skill", "desc", "original");
        let local_content = skill_content("local-skill", "desc", "locally modified");

        let fixture = TestFixture::new()
            .with_source_skill("local-skill", &source_content)
            .with_local_skill(Tool::Claude, "local-skill", &local_content);

        let catalog = fixture.catalog();
        let mut diagnostics = Diagnostics::new(false);
        let plans = collect_pull_plans(&catalog, None, &mut diagnostics).unwrap();

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].name, "local-skill");
        assert_eq!(plans[0].variants.len(), 1);
        assert_eq!(plans[0].variants[0].tool, Tool::Claude);
        assert_eq!(plans[0].variants[0].source, VariantSource::Local);
        assert!(!plans[0].variants[0].orphan);
    }

    #[test]
    fn detects_orphaned_local_skill() {
        let fixture = TestFixture::new()
            .with_local_skill(Tool::Codex, "local-orphan", &simple_skill("local-orphan"));

        let catalog = fixture.catalog();
        let mut diagnostics = Diagnostics::new(false);
        let plans = collect_pull_plans(&catalog, None, &mut diagnostics).unwrap();

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].name, "local-orphan");
        assert_eq!(plans[0].variants.len(), 1);
        assert_eq!(plans[0].variants[0].tool, Tool::Codex);
        assert_eq!(plans[0].variants[0].source, VariantSource::Local);
        assert!(plans[0].variants[0].orphan);
    }

    #[test]
    fn ignores_synced_skills() {
        let content = simple_skill("synced-skill");

        let fixture = TestFixture::new()
            .with_source_skill("synced-skill", &content)
            .with_tool_skill(Tool::Claude, "synced-skill", &content)
            .with_local_skill(Tool::Codex, "synced-skill", &content);

        let catalog = fixture.catalog();
        let mut diagnostics = Diagnostics::new(false);
        let plans = collect_pull_plans(&catalog, None, &mut diagnostics).unwrap();

        assert!(plans.is_empty());
    }

    #[test]
    fn filters_by_skill_name() {
        let fixture = TestFixture::new()
            .with_tool_skill(Tool::Claude, "skill-a", &simple_skill("skill-a"))
            .with_tool_skill(Tool::Claude, "skill-b", &simple_skill("skill-b"));

        let catalog = fixture.catalog();
        let mut diagnostics = Diagnostics::new(false);
        let plans = collect_pull_plans(&catalog, Some("skill-a"), &mut diagnostics).unwrap();

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].name, "skill-a");
    }

    #[test]
    fn errors_on_nonexistent_skill_filter() {
        let fixture = TestFixture::new()
            .with_tool_skill(Tool::Claude, "exists", &simple_skill("exists"));

        let catalog = fixture.catalog();
        let mut diagnostics = Diagnostics::new(false);
        let result = collect_pull_plans(&catalog, Some("nonexistent"), &mut diagnostics);

        assert!(result.is_err());
    }

    #[test]
    fn detects_multiple_variants_for_same_skill() {
        let source_content = skill_content("multi", "desc", "original");
        let global_modified = skill_content("multi", "desc", "global modified");
        let local_modified = skill_content("multi", "desc", "local modified");

        let fixture = TestFixture::new()
            .with_source_skill("multi", &source_content)
            .with_tool_skill(Tool::Claude, "multi", &global_modified)
            .with_local_skill(Tool::Claude, "multi", &local_modified);

        let catalog = fixture.catalog();
        let mut diagnostics = Diagnostics::new(false);
        let plans = collect_pull_plans(&catalog, None, &mut diagnostics).unwrap();

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].name, "multi");
        // Should have two variants: one global, one local
        assert_eq!(plans[0].variants.len(), 2);

        let has_global = plans[0]
            .variants
            .iter()
            .any(|v| v.source == VariantSource::Global);
        let has_local = plans[0]
            .variants
            .iter()
            .any(|v| v.source == VariantSource::Local);

        assert!(has_global, "Should have a global variant");
        assert!(has_local, "Should have a local variant");
    }

    #[test]
    fn detects_skills_across_multiple_tools() {
        let fixture = TestFixture::new()
            .with_tool_skill(Tool::Claude, "claude-only", &simple_skill("claude-only"))
            .with_tool_skill(Tool::Codex, "codex-only", &simple_skill("codex-only"))
            .with_local_skill(Tool::Claude, "local-claude", &simple_skill("local-claude"))
            .with_local_skill(Tool::Codex, "local-codex", &simple_skill("local-codex"));

        let catalog = fixture.catalog();
        let mut diagnostics = Diagnostics::new(false);
        let plans = collect_pull_plans(&catalog, None, &mut diagnostics).unwrap();

        assert_eq!(plans.len(), 4);

        let names: Vec<&str> = plans.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"claude-only"));
        assert!(names.contains(&"codex-only"));
        assert!(names.contains(&"local-claude"));
        assert!(names.contains(&"local-codex"));
    }
}
