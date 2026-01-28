//! Implementation of the `skills promote` command.

use std::fs;

use owo_colors::OwoColorize;

use crate::{
    catalog::Catalog,
    commands::{ColorChoice, init},
    config::Config,
    diagnostics::Diagnostics,
    error::{Error, Result},
    paths::display_path,
    skill::LocalSkill,
    tool::{Tool, ToolFilter},
};

/// Execute the promote command.
pub async fn run(
    color: ColorChoice,
    verbose: bool,
    skill_name: String,
    tool_filter: Option<ToolFilter>,
    dry_run: bool,
    force: bool,
) -> Result<()> {
    init::ensure().await?;
    let mut diagnostics = Diagnostics::new(verbose);
    let config = Config::load()?;
    let catalog = Catalog::load(&config, &mut diagnostics);
    let use_color = color.enabled();

    // Convert ToolFilter to Option<Tool> for filtering
    let tool_filter: Option<Tool> = match tool_filter {
        Some(ToolFilter::Claude) => Some(Tool::Claude),
        Some(ToolFilter::Codex) => Some(Tool::Codex),
        Some(ToolFilter::Gemini) => Some(Tool::Gemini),
        Some(ToolFilter::All) | None => None,
    };

    // Find matching local skills
    let matches = find_local_skills(&catalog, &skill_name, tool_filter);

    if matches.is_empty() {
        return Err(Error::LocalSkillNotFound { name: skill_name });
    }

    if matches.len() > 1 && tool_filter.is_none() {
        return Err(Error::AmbiguousLocalSkill { name: skill_name });
    }

    // Process each match (usually just one)
    for skill in matches {
        let target_dir = skill.tool.skills_dir()?;
        let target_skill_dir = target_dir.join(&skill.name);

        // Check if skill already exists at target
        if target_skill_dir.exists() && !force {
            return Err(Error::SkillExists {
                name: skill.name.clone(),
                path: target_skill_dir,
            });
        }

        // Print what we're doing
        let action = if dry_run { "Would promote" } else { "Promoting" };
        let from = display_path(&skill.skill_dir);
        let to = display_path(&target_skill_dir);

        if use_color {
            println!(
                "{} '{}' from {} to {}",
                action.bold(),
                skill.name.cyan(),
                from,
                to
            );
        } else {
            println!("{} '{}' from {} to {}", action, skill.name, from, to);
        }

        if dry_run {
            println!();
            println!("Dry run - no changes made.");
            continue;
        }

        // Ensure parent directory exists
        if let Some(parent) = target_skill_dir.parent()
            && !parent.exists()
        {
            fs::create_dir_all(parent).map_err(|e| Error::SkillMove {
                from: skill.skill_dir.clone(),
                to: target_skill_dir.clone(),
                source: e,
            })?;
        }

        // Remove existing if force
        if target_skill_dir.exists() && force {
            fs::remove_dir_all(&target_skill_dir).map_err(|e| Error::SkillMove {
                from: skill.skill_dir.clone(),
                to: target_skill_dir.clone(),
                source: e,
            })?;
        }

        // Move the skill directory
        fs::rename(&skill.skill_dir, &target_skill_dir).map_err(|e| Error::SkillMove {
            from: skill.skill_dir.clone(),
            to: target_skill_dir.clone(),
            source: e,
        })?;

        println!();
        println!("Done. To manage this skill from your source directory, run:");
        println!("  skills pull {} --to <source-dir>", skill.name);
    }

    diagnostics.print_skipped_summary();
    Ok(())
}

/// Find local skills matching the given name and optional tool filter.
fn find_local_skills<'a>(
    catalog: &'a Catalog,
    name: &str,
    tool_filter: Option<Tool>,
) -> Vec<&'a LocalSkill> {
    let mut matches = Vec::new();

    for (tool, skills) in &catalog.local {
        if let Some(filter) = tool_filter
            && *tool != filter
        {
            continue;
        }
        if let Some(skill) = skills.get(name) {
            matches.push(skill);
        }
    }

    matches
}
