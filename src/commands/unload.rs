//! Implementation of the `skills unload` command.

use std::fs;

use inquire::{Confirm, error::InquireError};

use crate::{
    commands::{ColorChoice, init},
    error::{Error, Result},
    skill::SKILL_FILE_NAME,
    tool::Tool,
};

/// Execute the unload command.
pub async fn run(
    _color: ColorChoice,
    _verbose: bool,
    skill: String,
    claude: bool,
    codex: bool,
    dry_run: bool,
    force: bool,
) -> Result<()> {
    init::ensure().await?;

    let tools = select_tools(claude, codex);
    let mut found_any = false;

    println!("Unloading {}...", skill);

    for tool in tools {
        let tool_dir = tool.skills_dir()?;
        let skill_dir = tool_dir.join(&skill);
        let skill_path = skill_dir.join(SKILL_FILE_NAME);

        if !skill_path.is_file() {
            println!("  {:<6}: - (not installed)", tool.id());
            continue;
        }

        found_any = true;

        if !force && !dry_run {
            let prompt = format!(
                "Remove skill '{}' from {}?",
                skill,
                tool.display_name()
            );
            let confirmed = confirm(&prompt)?;
            if !confirmed {
                println!("  {:<6}: ! (skipped)", tool.id());
                continue;
            }
        }

        if !dry_run {
            fs::remove_dir_all(&skill_dir).map_err(|e| Error::SkillWrite {
                path: skill_dir.clone(),
                source: e,
            })?;
        }

        println!("  {:<6}: - (removed)", tool.id());
    }

    if !found_any {
        println!("Skill '{}' is not installed in any tool.", skill);
    }

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
