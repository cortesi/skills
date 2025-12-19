//! Status computation for skills across tools.

use std::collections::BTreeSet;

use crate::{catalog::Catalog, diagnostics::Diagnostics, skill::render_template, tool::Tool};

/// Sync status for a skill in a tool directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncStatus {
    /// Tool copy matches the rendered source template.
    Synced,
    /// Tool copy differs from the rendered source template.
    Modified,
    /// Tool copy does not exist.
    Missing,
    /// Tool copy exists without a source skill.
    Orphan,
}

/// Status for a specific tool and skill.
#[derive(Debug, Clone, Copy)]
pub struct ToolStatus {
    /// Tool being described.
    pub(crate) tool: Tool,
    /// Status for the tool.
    pub(crate) status: SyncStatus,
}

/// Status entry for a skill across sources and tools.
#[derive(Debug, Clone)]
pub struct SkillEntry {
    /// Skill name.
    pub(crate) name: String,
    /// Statuses for each tool.
    pub(crate) tool_statuses: Vec<ToolStatus>,
}

/// Compute list entries with sync status across tools.
pub fn build_entries(catalog: &Catalog, diagnostics: &mut Diagnostics) -> Vec<SkillEntry> {
    let mut names = collect_names(catalog);
    let mut entries = Vec::new();
    for (index, name) in names.drain(..).enumerate() {
        let source = catalog.sources.get(&name);
        let mut tool_statuses = Vec::new();
        let mut skip = false;

        for tool in Tool::all() {
            let tool_map = catalog.tools.get(&tool);
            let tool_skill = tool_map.and_then(|skills| skills.get(&name));
            let status = match (source, tool_skill) {
                (Some(source), Some(tool_skill)) => {
                    let rendered = match render_template(&source.contents, tool) {
                        Ok(rendered) => rendered,
                        Err(error) => {
                            diagnostics.warn_skipped(&source.skill_path, error);
                            skip = true;
                            break;
                        }
                    };
                    if normalize_line_endings(&rendered)
                        == normalize_line_endings(&tool_skill.contents)
                    {
                        SyncStatus::Synced
                    } else {
                        SyncStatus::Modified
                    }
                }
                (Some(_), None) => SyncStatus::Missing,
                (None, Some(_)) => SyncStatus::Orphan,
                (None, None) => continue,
            };

            tool_statuses.push(ToolStatus { tool, status });
        }

        if skip {
            continue;
        }

        entries.push((
            index,
            SkillEntry {
                name,
                tool_statuses,
            },
        ));
    }

    sort_entries(&mut entries);
    entries.into_iter().map(|(_, entry)| entry).collect()
}

/// Normalize line endings and trailing newline for content comparisons.
pub fn normalize_line_endings(contents: &str) -> String {
    let mut normalized = contents.replace("\r\n", "\n").replace('\r', "\n");
    if normalized.ends_with('\n') {
        normalized.pop();
    }
    normalized
}

/// Sort skill entries case-insensitively, stable within ties.
fn sort_entries(entries: &mut [(usize, SkillEntry)]) {
    entries.sort_by(|(left_index, left), (right_index, right)| {
        let left_key = left.name.to_lowercase();
        let right_key = right.name.to_lowercase();
        left_key
            .cmp(&right_key)
            .then_with(|| left_index.cmp(right_index))
    });
}

/// Collect skill names in deterministic order before sorting.
fn collect_names(catalog: &Catalog) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut names = Vec::new();

    let mut source_names = catalog.sources.keys().cloned().collect::<Vec<_>>();
    source_names.sort();
    for name in source_names {
        if seen.insert(name.clone()) {
            names.push(name);
        }
    }

    let mut tool_names = Vec::new();
    for tools in catalog.tools.values() {
        for name in tools.keys() {
            tool_names.push(name.clone());
        }
    }
    tool_names.sort();
    for name in tool_names {
        if seen.insert(name.clone()) {
            names.push(name);
        }
    }

    names
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, time::SystemTime};

    use crate::{
        catalog::Catalog,
        diagnostics::Diagnostics,
        skill::{SkillTemplate, ToolSkill},
        status::{SyncStatus, build_entries, normalize_line_endings},
        tool::Tool,
    };

    fn sample_skill_template(name: &str) -> SkillTemplate {
        SkillTemplate {
            name: name.to_string(),
            source_root: "/tmp/source".into(),
            skill_dir: "/tmp/source/skill".into(),
            skill_path: "/tmp/source/skill/SKILL.md".into(),
            contents: "---\nname: sample\ndescription: desc\n---\n".to_string(),
        }
    }

    fn sample_tool_skill(name: &str, contents: &str) -> ToolSkill {
        ToolSkill {
            name: name.to_string(),
            skill_path: "/tmp/tool/skill/SKILL.md".into(),
            contents: contents.to_string(),
            modified: SystemTime::UNIX_EPOCH,
        }
    }

    #[test]
    fn normalizes_line_endings() {
        let normalized = normalize_line_endings("a\r\nb\r");
        assert_eq!(normalized, "a\nb");
    }

    #[test]
    fn preserves_extra_trailing_newlines() {
        let normalized = normalize_line_endings("a\n\n");
        assert_eq!(normalized, "a\n");
    }

    #[test]
    fn reports_modified_status() {
        let mut sources = HashMap::new();
        sources.insert("sample".to_string(), sample_skill_template("sample"));

        let mut tool_map = HashMap::new();
        tool_map.insert(
            "sample".to_string(),
            sample_tool_skill("sample", "---\nname: sample\ndescription: desc\n---\nextra"),
        );

        let mut tools = HashMap::new();
        tools.insert(Tool::Codex, tool_map);

        let catalog = Catalog { sources, tools };
        let mut diagnostics = Diagnostics::new(false);
        let entries = build_entries(&catalog, &mut diagnostics);

        let status = entries
            .iter()
            .find(|entry| entry.name == "sample")
            .and_then(|entry| {
                entry
                    .tool_statuses
                    .iter()
                    .find(|status| status.tool == Tool::Codex)
            })
            .map(|status| status.status);

        assert_eq!(status, Some(SyncStatus::Modified));
    }

    #[test]
    fn orders_entries_case_insensitively() {
        let mut sources = HashMap::new();
        sources.insert("beta".to_string(), sample_skill_template("beta"));
        sources.insert("Alpha".to_string(), sample_skill_template("Alpha"));

        let tools = HashMap::new();
        let catalog = Catalog { sources, tools };
        let mut diagnostics = Diagnostics::new(false);
        let entries = build_entries(&catalog, &mut diagnostics);

        let names = entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["Alpha", "beta"]);
    }
}
