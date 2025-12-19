//! Implementation of the `skills new` command.

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::error::{Error, Result};

/// Execute the new command.
pub async fn run(path: PathBuf) -> Result<()> {
    create_skill_template(&path)?;
    println!("Created skill at {}/SKILL.md", path.display());
    println!("\nEdit the SKILL.md file, then run `skills push` to sync.");
    Ok(())
}

/// Create a new skill directory and template.
fn create_skill_template(path: &Path) -> Result<()> {
    if path.exists() {
        return Err(Error::PathExists {
            path: path.to_path_buf(),
        });
    }

    let name = skill_name_from_path(path)?;
    let title = title_case(&name);

    fs::create_dir_all(path).map_err(|error| Error::SkillWrite {
        path: path.to_path_buf(),
        source: error,
    })?;

    let skill_path = path.join("SKILL.md");
    let template = format!(
        "---\nname: {name}\ndescription: <describe when this skill should be used>\n---\n\n# {title}\n\n<instructions for the AI assistant>\n"
    );

    fs::write(&skill_path, template).map_err(|error| Error::SkillWrite {
        path: skill_path,
        source: error,
    })?;

    Ok(())
}

/// Extract a skill name from the destination path.
fn skill_name_from_path(path: &Path) -> Result<String> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| Error::InvalidPath {
            path: path.to_path_buf(),
        })?;
    Ok(name.to_string())
}

/// Convert a hyphenated name into title case for headings.
fn title_case(name: &str) -> String {
    name.split('-')
        .filter(|part| !part.is_empty())
        .map(capitalize)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Capitalize the first character of a word.
fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
