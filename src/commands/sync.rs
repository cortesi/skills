//! Implementation of the `skills sync` command.

use std::{
    collections::HashMap,
    fs,
    path::Path,
    time::SystemTime,
};

use crate::{
    catalog::Catalog,
    commands::{ColorChoice, init},
    config::Config,
    diagnostics::Diagnostics,
    error::{Error, Result},
    palette::{fmt_label, fmt_skill_name, fmt_tool_tag},
    skill::{SKILL_FILE_NAME, SkillTemplate, ToolSkill, render_template},
    status::normalize_line_endings,
    tool::Tool,
};

/// Indent for subordinate information.
const INDENT: &str = "    ";

/// Conflict resolution strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    /// Error on conflict (default).
    Error,
    /// Prefer source version.
    PreferSource,
    /// Prefer tool version (newest).
    PreferTool,
}

/// Execute the sync command.
pub async fn run(
    color: ColorChoice,
    verbose: bool,
    skills: Vec<String>,
    prefer_source: bool,
    prefer_tool: bool,
    dry_run: bool,
) -> Result<()> {
    init::ensure().await?;
    let mut diagnostics = Diagnostics::new(verbose);
    let config = Config::load()?;
    let catalog = Catalog::load(&config, &mut diagnostics);
    let use_color = color.enabled();

    // Determine conflict resolution strategy
    let resolution = if prefer_source {
        ConflictResolution::PreferSource
    } else if prefer_tool {
        ConflictResolution::PreferTool
    } else {
        ConflictResolution::Error
    };

    // Validate specified skills exist
    if !skills.is_empty() {
        for name in &skills {
            if !catalog.sources.contains_key(name) {
                return Err(Error::SkillNotFound { name: name.clone() });
            }
        }
    }

    // Build sync plans for skills
    let mut plans = build_sync_plans(&catalog, &mut diagnostics)?;

    // Filter to specified skills if any
    if !skills.is_empty() {
        plans.retain(|p| skills.contains(&p.name));
    }

    if plans.is_empty() {
        println!("All skills are in sync.");
        return Ok(());
    }

    // Handle conflicts based on resolution strategy
    handle_conflicts(&mut plans, resolution)?;

    // Apply sync operations
    let mut push_count = 0;
    let mut pull_count = 0;

    for plan in &plans {
        match &plan.action {
            SyncAction::Push { to_tools } => {
                print_push(plan, to_tools, use_color);
                if !dry_run {
                    apply_push(plan, to_tools)?;
                }
                push_count += 1;
            }
            SyncAction::Pull { from_tool } => {
                print_pull(plan, *from_tool, use_color);
                if !dry_run {
                    apply_pull(plan, *from_tool)?;
                }
                pull_count += 1;
            }
            SyncAction::PullAndPush { from_tool, to_tools } => {
                print_pull_and_push(plan, *from_tool, to_tools, use_color);
                if !dry_run {
                    apply_pull(plan, *from_tool)?;
                    apply_push(plan, to_tools)?;
                }
                pull_count += 1;
                push_count += 1;
            }
        }
    }

    println!();
    if dry_run {
        println!(
            "{} {} push, {} pull operations would be performed.",
            fmt_label("Dry run:", use_color),
            push_count,
            pull_count
        );
    } else {
        println!(
            "{} {} pushed, {} pulled.",
            fmt_label("Synced:", use_color),
            push_count,
            pull_count
        );
    }

    diagnostics.print_skipped_summary();
    Ok(())
}

/// Print a push action.
fn print_push(plan: &SyncPlan, to_tools: &[Tool], use_color: bool) {
    println!("{}", fmt_skill_name(&plan.name, use_color));
    println!(
        "{}{} source -> {}",
        INDENT,
        fmt_label("sync:", use_color),
        format_tools(to_tools, use_color)
    );
}

/// Print a pull action.
fn print_pull(plan: &SyncPlan, from_tool: Tool, use_color: bool) {
    println!("{}", fmt_skill_name(&plan.name, use_color));
    println!(
        "{}{} {} -> source",
        INDENT,
        fmt_label("sync:", use_color),
        format_tool(from_tool, use_color)
    );
}

/// Print a pull-and-push action.
fn print_pull_and_push(plan: &SyncPlan, from_tool: Tool, to_tools: &[Tool], use_color: bool) {
    println!("{}", fmt_skill_name(&plan.name, use_color));
    println!(
        "{}{} {} -> source -> {}",
        INDENT,
        fmt_label("sync:", use_color),
        format_tool(from_tool, use_color),
        format_tools(to_tools, use_color)
    );
}

/// Sync plan for a single skill.
#[derive(Debug)]
struct SyncPlan {
    /// Skill name.
    name: String,
    /// Source skill template.
    source: SkillTemplate,
    /// Tool skills that differ from source.
    tool_skills: HashMap<Tool, ToolSkill>,
    /// Determined sync action.
    action: SyncAction,
}

/// Action to take for syncing a skill.
#[derive(Debug)]
enum SyncAction {
    /// Push source to tools.
    Push {
        /// Tools to push to.
        to_tools: Vec<Tool>,
    },
    /// Pull from tool to source.
    Pull {
        /// Tool to pull from.
        from_tool: Tool,
    },
    /// Pull from tool to source, then push to other tools.
    PullAndPush {
        /// Tool to pull from.
        from_tool: Tool,
        /// Tools to push to after pulling.
        to_tools: Vec<Tool>,
    },
}

/// Build sync plans for all skills that need syncing.
fn build_sync_plans(
    catalog: &Catalog,
    diagnostics: &mut Diagnostics,
) -> Result<Vec<SyncPlan>> {
    let mut plans = Vec::new();

    for (name, source) in &catalog.sources {
        let mut differing_tools: HashMap<Tool, ToolSkill> = HashMap::new();

        // Check each tool for differences
        for tool in Tool::all() {
            let tool_map = catalog.tools.get(&tool);
            let tool_skill = tool_map.and_then(|skills| skills.get(name));

            let Some(tool_skill) = tool_skill else {
                continue;
            };

            // Render the template for this tool
            let rendered = match render_template(&source.contents, tool) {
                Ok(rendered) => rendered,
                Err(error) => {
                    diagnostics.warn_skipped(&source.skill_path, error);
                    continue;
                }
            };

            // Check if contents differ
            if normalize_line_endings(&rendered) != normalize_line_endings(&tool_skill.contents) {
                differing_tools.insert(tool, tool_skill.clone());
            }
        }

        if differing_tools.is_empty() {
            continue;
        }

        // Determine sync action based on timestamps
        let action = determine_action(source, &differing_tools);

        plans.push(SyncPlan {
            name: name.clone(),
            source: source.clone(),
            tool_skills: differing_tools,
            action,
        });
    }

    // Sort by name for consistent output
    plans.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(plans)
}

/// Determine the sync action based on modification timestamps.
fn determine_action(source: &SkillTemplate, tool_skills: &HashMap<Tool, ToolSkill>) -> SyncAction {
    let source_mtime = source.modified;

    // Find the newest tool modification
    let mut newest_tool: Option<(Tool, SystemTime)> = None;
    for (tool, skill) in tool_skills {
        match &newest_tool {
            None => newest_tool = Some((*tool, skill.modified)),
            Some((_, mtime)) if skill.modified > *mtime => {
                newest_tool = Some((*tool, skill.modified));
            }
            _ => {}
        }
    }

    let Some((newest_tool_id, newest_tool_mtime)) = newest_tool else {
        // No tool skills, shouldn't happen but push to all
        return SyncAction::Push {
            to_tools: tool_skills.keys().copied().collect(),
        };
    };

    if source_mtime >= newest_tool_mtime {
        // Source is newer or equal, push to all differing tools
        SyncAction::Push {
            to_tools: tool_skills.keys().copied().collect(),
        }
    } else {
        // A tool is newer, pull from it
        let other_tools: Vec<Tool> = tool_skills
            .keys()
            .filter(|t| **t != newest_tool_id)
            .copied()
            .collect();

        if other_tools.is_empty() {
            SyncAction::Pull {
                from_tool: newest_tool_id,
            }
        } else {
            SyncAction::PullAndPush {
                from_tool: newest_tool_id,
                to_tools: other_tools,
            }
        }
    }
}

/// Handle conflicts based on resolution strategy.
fn handle_conflicts(plans: &mut Vec<SyncPlan>, resolution: ConflictResolution) -> Result<()> {
    for plan in plans.iter_mut() {
        if plan.tool_skills.len() < 2 {
            continue;
        }

        // Check if the tool skills differ from each other (not just from source)
        let skills: Vec<_> = plan.tool_skills.values().collect();
        let first_contents = normalize_line_endings(&skills[0].contents);

        let mut has_conflict = false;
        for skill in skills.iter().skip(1) {
            let contents = normalize_line_endings(&skill.contents);
            if contents != first_contents {
                has_conflict = true;
                break;
            }
        }

        if !has_conflict {
            continue;
        }

        // Tools have divergent modifications - handle based on resolution strategy
        match resolution {
            ConflictResolution::Error => {
                let tools: Vec<_> = plan
                    .tool_skills
                    .keys()
                    .map(|t| format!("[{}]", t.id()))
                    .collect();
                return Err(Error::SyncConflict {
                    name: plan.name.clone(),
                    tools: tools.join(" and "),
                });
            }
            ConflictResolution::PreferSource => {
                // Push source to all tools
                plan.action = SyncAction::Push {
                    to_tools: plan.tool_skills.keys().copied().collect(),
                };
            }
            ConflictResolution::PreferTool => {
                // Find newest tool and pull from it, push to others
                let newest_tool = plan
                    .tool_skills
                    .iter()
                    .max_by_key(|(_, s)| s.modified)
                    .map(|(t, _)| *t)
                    .unwrap();

                let other_tools: Vec<Tool> = plan
                    .tool_skills
                    .keys()
                    .filter(|t| **t != newest_tool)
                    .copied()
                    .collect();

                if other_tools.is_empty() {
                    plan.action = SyncAction::Pull { from_tool: newest_tool };
                } else {
                    plan.action = SyncAction::PullAndPush {
                        from_tool: newest_tool,
                        to_tools: other_tools,
                    };
                }
            }
        }
    }

    Ok(())
}

/// Apply a push operation.
fn apply_push(plan: &SyncPlan, to_tools: &[Tool]) -> Result<()> {
    for &tool in to_tools {
        let tool_dir = tool.skills_dir()?;
        let rendered = render_template(&plan.source.contents, tool)
            .map_err(|e| Error::TemplateRender { message: e })?;
        write_tool_skill(&tool_dir, &plan.name, &rendered)?;
    }
    Ok(())
}

/// Apply a pull operation.
fn apply_pull(plan: &SyncPlan, from_tool: Tool) -> Result<()> {
    let tool_skill = plan.tool_skills.get(&from_tool).ok_or_else(|| {
        Error::SkillNotFound {
            name: plan.name.clone(),
        }
    })?;

    let skill_path = plan.source.skill_path.clone();
    fs::write(&skill_path, &tool_skill.contents).map_err(|e| Error::SkillWrite {
        path: skill_path,
        source: e,
    })?;

    Ok(())
}

/// Write a skill to the tool directory.
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

/// Format a single tool for display with styling.
fn format_tool(tool: Tool, use_color: bool) -> String {
    let tag = format!("[{}]", tool.id());
    fmt_tool_tag(&tag, use_color)
}

/// Format a list of tools for display with styling.
fn format_tools(tools: &[Tool], use_color: bool) -> String {
    tools
        .iter()
        .map(|t| format_tool(*t, use_color))
        .collect::<Vec<_>>()
        .join(", ")
}
