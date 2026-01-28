//! Test utilities for setting up mock skill environments.
//!
//! This module provides a `TestFixture` builder for creating isolated test
//! environments with source, tool, and local skills.

#![allow(dead_code)]

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use tempfile::TempDir;

use crate::{
    catalog::Catalog,
    config::Config,
    diagnostics::Diagnostics,
    skill::{LocalSkill, SkillTemplate, ToolSkill, SKILL_FILE_NAME},
    tool::Tool,
};

/// Default skill content template with frontmatter.
pub fn skill_content(name: &str, description: &str, body: &str) -> String {
    format!(
        "---\nname: {}\ndescription: {}\n---\n{}",
        name, description, body
    )
}

/// Simple skill content with just name and description.
pub fn simple_skill(name: &str) -> String {
    skill_content(name, &format!("Description for {}", name), "")
}

/// Test fixture for creating isolated skill environments.
///
/// Creates temporary directories for source, tool, and local skills,
/// with a fluent builder API for populating them.
pub struct TestFixture {
    /// Root temp directory (holds everything).
    _root: TempDir,
    /// Source skills directory.
    source_dir: PathBuf,
    /// Tool skills directories keyed by tool.
    tool_dirs: HashMap<Tool, PathBuf>,
    /// Local skills directories keyed by tool (simulates project .claude/skills).
    local_dirs: HashMap<Tool, PathBuf>,
    /// Working directory (simulates project root).
    work_dir: PathBuf,
}

impl TestFixture {
    /// Create a new test fixture with empty directories.
    pub fn new() -> Self {
        let root = TempDir::new().expect("create temp dir");
        let root_path = root.path();

        let source_dir = root_path.join("sources");
        fs::create_dir_all(&source_dir).expect("create source dir");

        let work_dir = root_path.join("project");
        fs::create_dir_all(&work_dir).expect("create work dir");

        let mut tool_dirs = HashMap::new();
        let mut local_dirs = HashMap::new();

        for tool in Tool::all() {
            // Global tool directories (simulates ~/.claude/skills, ~/.codex/skills)
            let tool_dir = root_path.join(format!("global_{}", tool.id()));
            fs::create_dir_all(&tool_dir).expect("create tool dir");
            tool_dirs.insert(tool, tool_dir);

            // Local project directories (simulates .claude/skills, .codex/skills)
            let local_dir = work_dir.join(tool.local_skills_dir());
            fs::create_dir_all(&local_dir).expect("create local dir");
            local_dirs.insert(tool, local_dir);
        }

        Self {
            _root: root,
            source_dir,
            tool_dirs,
            local_dirs,
            work_dir,
        }
    }

    /// Add a source skill with the given name and contents.
    pub fn with_source_skill(self, name: &str, contents: &str) -> Self {
        let skill_dir = self.source_dir.join(name);
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        let skill_path = skill_dir.join(SKILL_FILE_NAME);
        fs::write(&skill_path, contents).expect("write skill");
        self
    }

    /// Add a global tool skill.
    pub fn with_tool_skill(self, tool: Tool, name: &str, contents: &str) -> Self {
        let tool_dir = self.tool_dirs.get(&tool).expect("tool dir exists");
        let skill_dir = tool_dir.join(name);
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        let skill_path = skill_dir.join(SKILL_FILE_NAME);
        fs::write(&skill_path, contents).expect("write skill");
        self
    }

    /// Add a local project skill.
    pub fn with_local_skill(self, tool: Tool, name: &str, contents: &str) -> Self {
        let local_dir = self.local_dirs.get(&tool).expect("local dir exists");
        let skill_dir = local_dir.join(name);
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        let skill_path = skill_dir.join(SKILL_FILE_NAME);
        fs::write(&skill_path, contents).expect("write skill");
        self
    }

    /// Get the source directory path.
    pub fn source_dir(&self) -> &Path {
        &self.source_dir
    }

    /// Get the tool directory path for a specific tool.
    pub fn tool_dir(&self, tool: Tool) -> &Path {
        self.tool_dirs.get(&tool).expect("tool dir exists")
    }

    /// Get the local skills directory path for a specific tool.
    pub fn local_dir(&self, tool: Tool) -> &Path {
        self.local_dirs.get(&tool).expect("local dir exists")
    }

    /// Get the work directory path (project root).
    pub fn work_dir(&self) -> &Path {
        &self.work_dir
    }

    /// Build a Config pointing to the test source directory.
    pub fn config(&self) -> Config {
        Config::new(vec![self.source_dir.clone()])
    }

    /// Load a Catalog from the test directories.
    ///
    /// This directly constructs the catalog from the test directories,
    /// bypassing the normal loading which looks at real home directories.
    pub fn catalog(&self) -> Catalog {
        self.catalog_with_diagnostics(&mut Diagnostics::new(false))
    }

    /// Load a Catalog with custom diagnostics.
    pub fn catalog_with_diagnostics(&self, diagnostics: &mut Diagnostics) -> Catalog {
        let sources = self.load_sources(diagnostics);
        let tools = self.load_tools(diagnostics);
        let local = self.load_local(diagnostics);
        Catalog::new(sources, tools, local)
    }

    /// Load source skills from the test source directory.
    fn load_sources(&self, diagnostics: &mut Diagnostics) -> HashMap<String, SkillTemplate> {
        let mut skills = HashMap::new();
        let entries = match fs::read_dir(&self.source_dir) {
            Ok(entries) => entries,
            Err(_) => return skills,
        };

        for entry in entries.filter_map(|e| e.ok()) {
            let skill_dir = entry.path();
            if let Some(skill) =
                crate::skill::load_source_skill(&self.source_dir, &skill_dir, diagnostics)
            {
                skills.insert(skill.name.clone(), skill);
            }
        }
        skills
    }

    /// Load tool skills from the test tool directories.
    fn load_tools(&self, diagnostics: &mut Diagnostics) -> HashMap<Tool, HashMap<String, ToolSkill>> {
        let mut tools = HashMap::new();
        for tool in Tool::all() {
            let tool_dir = self.tool_dirs.get(&tool).expect("tool dir");
            let mut skills = HashMap::new();

            let entries = match fs::read_dir(tool_dir) {
                Ok(entries) => entries,
                Err(_) => {
                    tools.insert(tool, skills);
                    continue;
                }
            };

            for entry in entries.filter_map(|e| e.ok()) {
                let skill_dir = entry.path();
                if let Some(skill) = crate::skill::load_tool_skill(&skill_dir, diagnostics) {
                    skills.insert(skill.name.clone(), skill);
                }
            }
            tools.insert(tool, skills);
        }
        tools
    }

    /// Load local skills from the test local directories.
    fn load_local(&self, diagnostics: &mut Diagnostics) -> HashMap<Tool, HashMap<String, LocalSkill>> {
        let mut local = HashMap::new();
        for tool in Tool::all() {
            let local_dir = self.local_dirs.get(&tool).expect("local dir");
            let mut skills = HashMap::new();

            let entries = match fs::read_dir(local_dir) {
                Ok(entries) => entries,
                Err(_) => {
                    local.insert(tool, skills);
                    continue;
                }
            };

            for entry in entries.filter_map(|e| e.ok()) {
                let skill_dir = entry.path();
                if let Some(skill) = crate::skill::load_local_skill(&skill_dir, tool, diagnostics) {
                    skills.insert(skill.name.clone(), skill);
                }
            }
            local.insert(tool, skills);
        }
        local
    }

    /// Read the contents of a source skill.
    pub fn read_source_skill(&self, name: &str) -> Option<String> {
        let path = self.source_dir.join(name).join(SKILL_FILE_NAME);
        fs::read_to_string(path).ok()
    }

    /// Read the contents of a tool skill.
    pub fn read_tool_skill(&self, tool: Tool, name: &str) -> Option<String> {
        let tool_dir = self.tool_dirs.get(&tool)?;
        let path = tool_dir.join(name).join(SKILL_FILE_NAME);
        fs::read_to_string(path).ok()
    }

    /// Read the contents of a local skill.
    pub fn read_local_skill(&self, tool: Tool, name: &str) -> Option<String> {
        let local_dir = self.local_dirs.get(&tool)?;
        let path = local_dir.join(name).join(SKILL_FILE_NAME);
        fs::read_to_string(path).ok()
    }

    /// Check if a source skill exists.
    pub fn source_skill_exists(&self, name: &str) -> bool {
        self.source_dir.join(name).join(SKILL_FILE_NAME).exists()
    }

    /// Check if a tool skill exists.
    pub fn tool_skill_exists(&self, tool: Tool, name: &str) -> bool {
        self.tool_dirs
            .get(&tool)
            .map(|d| d.join(name).join(SKILL_FILE_NAME).exists())
            .unwrap_or(false)
    }

    /// Check if a local skill exists.
    pub fn local_skill_exists(&self, tool: Tool, name: &str) -> bool {
        self.local_dirs
            .get(&tool)
            .map(|d| d.join(name).join(SKILL_FILE_NAME).exists())
            .unwrap_or(false)
    }
}

impl Default for TestFixture {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_empty_fixture() {
        let fixture = TestFixture::new();
        assert!(fixture.source_dir().exists());
        assert!(fixture.tool_dir(Tool::Claude).exists());
        assert!(fixture.tool_dir(Tool::Codex).exists());
        assert!(fixture.tool_dir(Tool::Gemini).exists());
        assert!(fixture.local_dir(Tool::Claude).exists());
        assert!(fixture.local_dir(Tool::Codex).exists());
        assert!(fixture.local_dir(Tool::Gemini).exists());
    }

    #[test]
    fn adds_source_skill() {
        let fixture = TestFixture::new()
            .with_source_skill("my-skill", &simple_skill("my-skill"));

        assert!(fixture.source_skill_exists("my-skill"));
        let contents = fixture.read_source_skill("my-skill").unwrap();
        assert!(contents.contains("name: my-skill"));
    }

    #[test]
    fn adds_tool_skill() {
        let fixture = TestFixture::new()
            .with_tool_skill(Tool::Claude, "my-skill", &simple_skill("my-skill"));

        assert!(fixture.tool_skill_exists(Tool::Claude, "my-skill"));
        assert!(!fixture.tool_skill_exists(Tool::Codex, "my-skill"));
    }

    #[test]
    fn adds_local_skill() {
        let fixture = TestFixture::new()
            .with_local_skill(Tool::Codex, "my-skill", &simple_skill("my-skill"));

        assert!(fixture.local_skill_exists(Tool::Codex, "my-skill"));
        assert!(!fixture.local_skill_exists(Tool::Claude, "my-skill"));
    }

    #[test]
    fn loads_catalog_with_all_skill_types() {
        let fixture = TestFixture::new()
            .with_source_skill("source-skill", &simple_skill("source-skill"))
            .with_tool_skill(Tool::Claude, "tool-skill", &simple_skill("tool-skill"))
            .with_local_skill(Tool::Codex, "local-skill", &simple_skill("local-skill"));

        let catalog = fixture.catalog();

        assert!(catalog.sources.contains_key("source-skill"));
        assert!(catalog.tools.get(&Tool::Claude).unwrap().contains_key("tool-skill"));
        assert!(catalog.local.get(&Tool::Codex).unwrap().contains_key("local-skill"));
    }
}
