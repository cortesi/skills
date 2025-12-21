//! Implementation of the `skills mv` command.

use std::fs;

use inquire::{Confirm, error::InquireError};
use owo_colors::OwoColorize;

use crate::{
    catalog::Catalog,
    commands::{ColorChoice, init},
    config::Config,
    diagnostics::Diagnostics,
    error::{Error, Result},
    paths::display_path,
    skill::SKILL_FILE_NAME,
    tool::Tool,
};

/// Execute the mv command.
pub async fn run(
    color: ColorChoice,
    verbose: bool,
    old_name: String,
    new_name: String,
    dry_run: bool,
    force: bool,
) -> Result<()> {
    init::ensure().await?;
    let mut diagnostics = Diagnostics::new(verbose);
    let config = Config::load()?;
    let catalog = Catalog::load(&config, &mut diagnostics);
    let use_color = color.enabled();

    // Check if old skill exists
    let source_skill = catalog.sources.get(&old_name);
    if source_skill.is_none() {
        return Err(Error::SkillNotFound {
            name: old_name.clone(),
        });
    }

    let source_skill = source_skill.unwrap();
    let old_source_dir = source_skill.skill_dir.clone();
    let new_source_dir = old_source_dir.parent().unwrap().join(&new_name);

    // Check if new name already exists
    if catalog.sources.contains_key(&new_name) && !force {
        return Err(Error::SkillExists {
            name: new_name.clone(),
            path: new_source_dir.clone(),
        });
    }

    // Print what we're doing
    if use_color {
        println!(
            "{} '{}' -> '{}'",
            if dry_run { "Would rename" } else { "Renaming" }.bold(),
            old_name.cyan(),
            new_name.cyan()
        );
    } else {
        println!(
            "{} '{}' -> '{}'",
            if dry_run { "Would rename" } else { "Renaming" },
            old_name,
            new_name
        );
    }

    // Collect all locations to rename
    let mut rename_ops = Vec::new();

    // Source directory
    rename_ops.push((old_source_dir.clone(), new_source_dir.clone(), "source"));

    // Tool directories
    for tool in Tool::all() {
        if let Some(skills) = catalog.tools.get(&tool) {
            if skills.contains_key(&old_name) {
                let tool_dir = tool.skills_dir()?;
                let old_tool_dir = tool_dir.join(&old_name);
                let new_tool_dir = tool_dir.join(&new_name);
                rename_ops.push((old_tool_dir, new_tool_dir, tool.id()));
            }
        }
    }

    // Print locations
    println!();
    for (old_path, new_path, label) in &rename_ops {
        println!(
            "  {}: {} -> {}",
            label,
            display_path(old_path),
            display_path(new_path)
        );
    }

    if dry_run {
        println!();
        println!("Dry run - no changes made.");
        return Ok(());
    }

    // Confirm if not forced
    if !force {
        println!();
        let confirmed = confirm(&format!("Rename {} location(s)?", rename_ops.len()))?;
        if !confirmed {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Perform renames
    for (old_path, new_path, _label) in &rename_ops {
        // Remove destination if it exists and force is set
        if new_path.exists() && force {
            fs::remove_dir_all(new_path).map_err(|e| Error::SkillMove {
                from: old_path.clone(),
                to: new_path.clone(),
                source: e,
            })?;
        }

        fs::rename(old_path, new_path).map_err(|e| Error::SkillMove {
            from: old_path.clone(),
            to: new_path.clone(),
            source: e,
        })?;
    }

    // Update the SKILL.md frontmatter name field in source
    let new_skill_path = new_source_dir.join(SKILL_FILE_NAME);
    if new_skill_path.exists() {
        let contents = fs::read_to_string(&new_skill_path).map_err(|e| Error::SkillRead {
            path: new_skill_path.clone(),
            source: e,
        })?;

        // Update the name field in frontmatter
        let updated = update_frontmatter_name(&contents, &new_name);
        fs::write(&new_skill_path, updated).map_err(|e| Error::SkillWrite {
            path: new_skill_path,
            source: e,
        })?;
    }

    println!();
    println!("Done. Renamed {} location(s).", rename_ops.len());

    diagnostics.print_skipped_summary();
    Ok(())
}

/// Update the name field in YAML frontmatter.
fn update_frontmatter_name(contents: &str, new_name: &str) -> String {
    // Find frontmatter boundaries
    let Some(start) = contents.find("---") else {
        return contents.to_string();
    };
    let rest = &contents[start + 3..];
    let Some(end) = rest.find("---") else {
        return contents.to_string();
    };

    let frontmatter = &rest[..end];
    let after = &rest[end..];

    // Replace name field
    let updated_frontmatter = frontmatter
        .lines()
        .map(|line| {
            if line.trim_start().starts_with("name:") {
                format!("name: {}", new_name)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!("---{}{}", updated_frontmatter, after)
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
