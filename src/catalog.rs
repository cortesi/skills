//! Skill catalog loading from sources and tools.

use std::{
    collections::HashMap,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use crate::{
    config::Config,
    diagnostics::Diagnostics,
    paths::display_path,
    skill::{SkillTemplate, ToolSkill, load_source_skill, load_tool_skill},
    tool::Tool,
};

/// In-memory catalog of source and tool skills.
#[derive(Debug)]
pub struct Catalog {
    /// Loaded source skills keyed by name.
    pub(crate) sources: HashMap<String, SkillTemplate>,
    /// Loaded tool skills keyed by tool and name.
    pub(crate) tools: HashMap<Tool, HashMap<String, ToolSkill>>,
}

impl Catalog {
    /// Load sources and tool installs into a catalog.
    pub(crate) fn load(config: &Config, diagnostics: &mut Diagnostics) -> Self {
        let sources = load_sources(config, diagnostics);
        let tools = load_tools(diagnostics);
        Self { sources, tools }
    }
}

/// Load source skills from configured directories.
fn load_sources(config: &Config, diagnostics: &mut Diagnostics) -> HashMap<String, SkillTemplate> {
    let mut skills: HashMap<String, SkillTemplate> = HashMap::new();
    let mut conflicts: HashMap<String, Vec<PathBuf>> = HashMap::new();

    for source_root in config.sources() {
        let entries = match read_source_directory(source_root, diagnostics) {
            Some(entries) => entries,
            None => continue,
        };

        for entry in entries {
            let skill_dir = entry.path();
            let Some(skill) = load_source_skill(source_root, &skill_dir, diagnostics) else {
                continue;
            };

            if let Some(existing) = skills.get(&skill.name) {
                let list = conflicts
                    .entry(skill.name.clone())
                    .or_insert_with(|| vec![existing.skill_dir.clone()]);
                list.push(skill.skill_dir.clone());
                continue;
            }

            skills.insert(skill.name.clone(), skill);
        }
    }

    emit_conflicts(&conflicts, &skills, diagnostics);
    skills
}

/// Load tool-installed skills for all supported tools.
fn load_tools(diagnostics: &mut Diagnostics) -> HashMap<Tool, HashMap<String, ToolSkill>> {
    let mut tools = HashMap::new();

    for tool in Tool::all() {
        let dir = match tool.skills_dir() {
            Ok(dir) => dir,
            Err(error) => {
                diagnostics.warn(error.to_string());
                continue;
            }
        };
        let entries = read_tool_directory(&dir, diagnostics);

        let mut skills = HashMap::new();
        for entry in entries {
            let skill_dir = entry.path();
            let Some(skill) = load_tool_skill(&skill_dir, diagnostics) else {
                continue;
            };
            skills.insert(skill.name.clone(), skill);
        }
        tools.insert(tool, skills);
    }

    tools
}

/// Read a source directory and return sorted entries.
fn read_source_directory(path: &Path, diagnostics: &mut Diagnostics) -> Option<Vec<fs::DirEntry>> {
    let mut entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            diagnostics.warn(format!("source directory not found: {}", path.display()));
            return None;
        }
        Err(error) => {
            diagnostics.warn(format!(
                "failed to read directory {}: {error}",
                path.display()
            ));
            return None;
        }
    }
    .filter_map(|entry| entry.ok())
    .collect::<Vec<_>>();

    entries.sort_by_key(|entry| entry.file_name());
    Some(entries)
}

/// Read a tool directory and return sorted entries, returning empty on missing.
fn read_tool_directory(path: &Path, diagnostics: &mut Diagnostics) -> Vec<fs::DirEntry> {
    let mut entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Vec::new(),
        Err(error) => {
            diagnostics.warn(format!(
                "failed to read directory {}: {error}",
                path.display()
            ));
            return Vec::new();
        }
    }
    .filter_map(|entry| entry.ok())
    .collect::<Vec<_>>();

    entries.sort_by_key(|entry| entry.file_name());
    entries
}

/// Emit warnings for source conflicts.
fn emit_conflicts(
    conflicts: &HashMap<String, Vec<PathBuf>>,
    skills: &HashMap<String, SkillTemplate>,
    diagnostics: &mut Diagnostics,
) {
    for (name, paths) in conflicts {
        let Some(primary) = skills.get(name) else {
            continue;
        };
        diagnostics.warn(format!(
            "skill '{name}' exists in multiple sources, using {}",
            display_path(&primary.source_root)
        ));
        for path in paths {
            diagnostics.note(format!("  - {}", display_path(path)));
        }
    }
}
