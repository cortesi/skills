//! Implementation of the `skills edit` command.

use std::{env, process::Command};

use crate::{
    catalog::Catalog,
    commands::init,
    config::Config,
    diagnostics::Diagnostics,
    error::{Error, Result},
    skill::SKILL_FILE_NAME,
    tool::Tool,
};

/// Execute the edit command.
pub async fn run(verbose: bool, skill_name: String) -> Result<()> {
    init::ensure().await?;
    let mut diagnostics = Diagnostics::new(verbose);
    let config = Config::load()?;
    let catalog = Catalog::load(&config, &mut diagnostics);

    // Find the skill file path - search sources first, then tools, then local
    let skill_path = if let Some(source) = catalog.sources.get(&skill_name) {
        source.skill_path.clone()
    } else {
        // Check tool installations
        let mut found = None;
        for tool in Tool::all() {
            if let Some(skills) = catalog.tools.get(&tool) {
                if let Some(skill) = skills.get(&skill_name) {
                    found = Some(skill.skill_path.clone());
                    break;
                }
            }
        }
        // Check local skills
        if found.is_none() {
            for skills in catalog.local.values() {
                if let Some(skill) = skills.get(&skill_name) {
                    found = Some(skill.skill_dir.join(SKILL_FILE_NAME));
                    break;
                }
            }
        }
        found.ok_or_else(|| Error::SkillNotFound {
            name: skill_name.clone(),
        })?
    };

    // Get editor from environment
    let editor = env::var("EDITOR")
        .or_else(|_| env::var("VISUAL"))
        .unwrap_or_else(|_| "vi".to_string());

    // Open the skill in the editor
    let status = Command::new(&editor)
        .arg(&skill_path)
        .status()
        .map_err(|e| Error::EditorFailed {
            editor: editor.clone(),
            message: e.to_string(),
        })?;

    if !status.success() {
        return Err(Error::EditorFailed {
            editor,
            message: format!("exited with status {}", status),
        });
    }

    Ok(())
}
