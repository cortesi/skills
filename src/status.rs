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
    use crate::{
        diagnostics::Diagnostics,
        status::{SyncStatus, build_entries, normalize_line_endings},
        testutil::{TestFixture, simple_skill, skill_content},
        tool::Tool,
    };

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
        let source_content = skill_content("sample", "desc", "");
        let tool_content = skill_content("sample", "desc", "extra");

        let fixture = TestFixture::new()
            .with_source_skill("sample", &source_content)
            .with_tool_skill(Tool::Codex, "sample", &tool_content);

        let catalog = fixture.catalog();
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
    fn reports_synced_status() {
        let content = simple_skill("synced");

        let fixture = TestFixture::new()
            .with_source_skill("synced", &content)
            .with_tool_skill(Tool::Claude, "synced", &content);

        let catalog = fixture.catalog();
        let mut diagnostics = Diagnostics::new(false);
        let entries = build_entries(&catalog, &mut diagnostics);

        let status = entries
            .iter()
            .find(|entry| entry.name == "synced")
            .and_then(|entry| {
                entry
                    .tool_statuses
                    .iter()
                    .find(|status| status.tool == Tool::Claude)
            })
            .map(|status| status.status);

        assert_eq!(status, Some(SyncStatus::Synced));
    }

    #[test]
    fn reports_missing_status() {
        let fixture = TestFixture::new()
            .with_source_skill("missing", &simple_skill("missing"));

        let catalog = fixture.catalog();
        let mut diagnostics = Diagnostics::new(false);
        let entries = build_entries(&catalog, &mut diagnostics);

        let status = entries
            .iter()
            .find(|entry| entry.name == "missing")
            .and_then(|entry| {
                entry
                    .tool_statuses
                    .iter()
                    .find(|status| status.tool == Tool::Claude)
            })
            .map(|status| status.status);

        assert_eq!(status, Some(SyncStatus::Missing));
    }

    #[test]
    fn reports_orphan_status() {
        let fixture = TestFixture::new()
            .with_tool_skill(Tool::Codex, "orphan", &simple_skill("orphan"));

        let catalog = fixture.catalog();
        let mut diagnostics = Diagnostics::new(false);
        let entries = build_entries(&catalog, &mut diagnostics);

        let status = entries
            .iter()
            .find(|entry| entry.name == "orphan")
            .and_then(|entry| {
                entry
                    .tool_statuses
                    .iter()
                    .find(|status| status.tool == Tool::Codex)
            })
            .map(|status| status.status);

        assert_eq!(status, Some(SyncStatus::Orphan));
    }

    #[test]
    fn orders_entries_case_insensitively() {
        let fixture = TestFixture::new()
            .with_source_skill("beta", &simple_skill("beta"))
            .with_source_skill("Alpha", &simple_skill("Alpha"));

        let catalog = fixture.catalog();
        let mut diagnostics = Diagnostics::new(false);
        let entries = build_entries(&catalog, &mut diagnostics);

        let names = entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["Alpha", "beta"]);
    }
}
