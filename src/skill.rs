//! Skill loading and templating helpers.

use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use minijinja::{Environment, UndefinedBehavior, context};

use crate::{diagnostics::Diagnostics, frontmatter::parse_frontmatter, tool::Tool};

/// The expected skill file name within a skill directory.
pub const SKILL_FILE_NAME: &str = "SKILL.md";

/// Source skill metadata and template contents.
#[derive(Debug, Clone)]
pub struct SkillTemplate {
    /// Skill name from frontmatter.
    pub(crate) name: String,
    /// Root source directory for this skill.
    pub(crate) source_root: PathBuf,
    /// Directory containing the skill file.
    pub(crate) skill_dir: PathBuf,
    /// Path to the skill file.
    pub(crate) skill_path: PathBuf,
    /// Raw template contents of the skill file.
    pub(crate) contents: String,
}

/// Installed tool skill metadata and contents.
#[derive(Debug, Clone)]
pub struct ToolSkill {
    /// Skill name from frontmatter.
    pub(crate) name: String,
    /// Path to the skill file.
    pub(crate) skill_path: PathBuf,
    /// Raw contents of the skill file.
    pub(crate) contents: String,
    /// Modified time for the skill file.
    pub(crate) modified: SystemTime,
}

/// Local skill in a project directory (.claude/skills or .codex/skills).
#[derive(Debug, Clone)]
pub struct LocalSkill {
    /// Skill name from frontmatter.
    pub(crate) name: String,
    /// Which tool this local skill belongs to (Claude or Codex).
    pub(crate) tool: Tool,
    /// Directory containing the skill file.
    pub(crate) skill_dir: PathBuf,
}

/// Load a source skill from a directory if present.
pub fn load_source_skill(
    source_root: &Path,
    skill_dir: &Path,
    diagnostics: &mut Diagnostics,
) -> Option<SkillTemplate> {
    let skill_path = skill_dir.join(SKILL_FILE_NAME);
    if !skill_path.is_file() {
        return None;
    }

    let contents = match fs::read_to_string(&skill_path) {
        Ok(contents) => contents,
        Err(error) => {
            diagnostics.warn_skipped(&skill_path, error.to_string());
            return None;
        }
    };

    let frontmatter = match parse_frontmatter(&contents) {
        Ok(frontmatter) => frontmatter,
        Err(error) => {
            diagnostics.warn_skipped(&skill_path, error.message);
            return None;
        }
    };

    Some(SkillTemplate {
        name: frontmatter.name,
        source_root: source_root.to_path_buf(),
        skill_dir: skill_dir.to_path_buf(),
        skill_path,
        contents,
    })
}

/// Load a tool-installed skill from a directory if present.
pub fn load_tool_skill(skill_dir: &Path, diagnostics: &mut Diagnostics) -> Option<ToolSkill> {
    let skill_path = skill_dir.join(SKILL_FILE_NAME);
    if !skill_path.is_file() {
        return None;
    }

    let contents = match fs::read_to_string(&skill_path) {
        Ok(contents) => contents,
        Err(error) => {
            diagnostics.warn_skipped(&skill_path, error.to_string());
            return None;
        }
    };

    let frontmatter = match parse_frontmatter(&contents) {
        Ok(frontmatter) => frontmatter,
        Err(error) => {
            diagnostics.warn_skipped(&skill_path, error.message);
            return None;
        }
    };

    let modified = fs::metadata(&skill_path)
        .and_then(|metadata| metadata.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);

    Some(ToolSkill {
        name: frontmatter.name,
        skill_path,
        contents,
        modified,
    })
}

/// Load a local skill from a project directory if present.
pub fn load_local_skill(
    skill_dir: &Path,
    tool: Tool,
    diagnostics: &mut Diagnostics,
) -> Option<LocalSkill> {
    let skill_path = skill_dir.join(SKILL_FILE_NAME);
    if !skill_path.is_file() {
        return None;
    }

    let contents = match fs::read_to_string(&skill_path) {
        Ok(contents) => contents,
        Err(error) => {
            diagnostics.warn_skipped(&skill_path, error.to_string());
            return None;
        }
    };

    let frontmatter = match parse_frontmatter(&contents) {
        Ok(frontmatter) => frontmatter,
        Err(error) => {
            diagnostics.warn_skipped(&skill_path, error.message);
            return None;
        }
    };

    Some(LocalSkill {
        name: frontmatter.name,
        tool,
        skill_dir: skill_dir.to_path_buf(),
    })
}

/// Render a skill template for a specific tool.
pub fn render_template(template: &str, tool: Tool) -> Result<String, String> {
    let mut env = Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::Strict);
    let template = env
        .template_from_str(template)
        .map_err(|error| error.to_string())?;
    template
        .render(context! { tool => tool.id() })
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use crate::{skill::render_template, tool::Tool};

    #[test]
    fn renders_tool_specific_templates() {
        let template = "{% if tool == \"codex\" %}Codex{% endif %}";
        let rendered = render_template(template, Tool::Codex).expect("rendered");
        assert_eq!(rendered, "Codex");
    }
}
