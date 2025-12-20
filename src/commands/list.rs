//! Implementation of the `skills list` command.

use std::{collections::HashSet, env, path::Path};

use owo_colors::OwoColorize;
use textwrap::{Options, wrap};

use crate::{
    catalog::Catalog,
    commands::{ColorChoice, init},
    config::Config,
    diagnostics::Diagnostics,
    error::Result,
    paths::display_path,
    skill::LocalSkill,
    status::{SkillEntry, SyncStatus, build_entries},
    tool::Tool,
};

/// Execute the list command.
pub async fn run(color: ColorChoice, verbose: bool) -> Result<()> {
    init::ensure().await?;
    let mut diagnostics = Diagnostics::new(verbose);
    let config = Config::load()?;
    let catalog = Catalog::load(&config, &mut diagnostics);
    let entries = build_entries(&catalog, &mut diagnostics);
    let use_color = color.enabled();

    // Print source/tool skills
    for entry in &entries {
        let skill = catalog.sources.get(&entry.name);
        let source_path = skill
            .map(|s| display_path(&s.source_root))
            .unwrap_or_else(|| "-".to_string());
        let description = skill.map(|s| s.description.as_str()).unwrap_or("-");

        let claude = format_status(status_for_tool(entry, Tool::Claude), use_color);
        let codex = format_status(status_for_tool(entry, Tool::Codex), use_color);

        println!("{}", entry.name);
        println!("{}", wrap_text(description, "  "));
        println!("  source: {}", source_path);
        println!("  claude: {:<9} codex: {:<9}", claude, codex);
        println!();
    }

    // Collect and print local skills
    let local_skills = collect_local_skills(&catalog);
    let cwd = env::current_dir().ok();
    if !local_skills.is_empty() {
        if use_color {
            println!("{}", "Local Skills:".bold());
        } else {
            println!("Local Skills:");
        }
        println!();
        for skill in &local_skills {
            let tool_label = format!("[{}]", skill.tool.id());
            let path_display = display_relative_path(&skill.skill_dir, cwd.as_deref());
            println!("  {} {}", skill.name, tool_label.dimmed());
            println!("{}", wrap_text(&skill.description, "    "));
            println!("    path: {}", path_display);
            println!();
        }
    }

    // Print conflicts between local and global skills
    let conflicts = find_conflicts(&catalog);
    if !conflicts.is_empty() {
        if use_color {
            println!("{}", "Conflicts:".bold().yellow());
        } else {
            println!("Conflicts:");
        }
        println!();
        for (name, tool) in &conflicts {
            let warning = format!(
                "  ⚠ '{}' exists locally and in {} global skills",
                name,
                tool.id()
            );
            if use_color {
                println!("{}", warning.yellow());
            } else {
                println!("{}", warning);
            }
            println!("    Local takes precedence in this project");
            println!();
        }
    }

    if entries.is_empty() && local_skills.is_empty() {
        println!("No skills found.");
        println!();
    }

    diagnostics.print_skipped_summary();
    Ok(())
}

/// Find the status for a tool within an entry.
fn status_for_tool(entry: &SkillEntry, tool: Tool) -> SyncStatus {
    entry
        .tool_statuses
        .iter()
        .find(|status| status.tool == tool)
        .map(|status| status.status)
        .unwrap_or(SyncStatus::Missing)
}

/// Format a status string with optional color.
fn format_status(status: SyncStatus, color: bool) -> String {
    let label = match status {
        SyncStatus::Synced => "synced",
        SyncStatus::Modified => "modified",
        SyncStatus::Missing => "missing",
        SyncStatus::Orphan => "orphan",
    };

    if !color {
        return label.to_string();
    }

    match status {
        SyncStatus::Synced => label.green().to_string(),
        SyncStatus::Modified => label.yellow().to_string(),
        SyncStatus::Missing => label.red().to_string(),
        SyncStatus::Orphan => label.red().to_string(),
    }
}

/// Collect all local skills from the catalog, sorted by name.
fn collect_local_skills(catalog: &Catalog) -> Vec<&LocalSkill> {
    let mut skills: Vec<&LocalSkill> = catalog
        .local
        .values()
        .flat_map(|skills| skills.values())
        .collect();
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

/// Find conflicts where a local skill shadows a global tool skill.
fn find_conflicts(catalog: &Catalog) -> Vec<(String, Tool)> {
    let mut conflicts = Vec::new();
    let mut seen: HashSet<(String, Tool)> = HashSet::new();

    for (tool, local_skills) in &catalog.local {
        if let Some(tool_skills) = catalog.tools.get(tool) {
            for name in local_skills.keys() {
                if tool_skills.contains_key(name) && seen.insert((name.clone(), *tool)) {
                    conflicts.push((name.clone(), *tool));
                }
            }
        }
    }

    conflicts.sort_by(|a, b| a.0.cmp(&b.0));
    conflicts
}

/// Maximum width for wrapped text.
const WRAP_WIDTH: usize = 80;

/// Wrap text to a given width with an indent prefix.
fn wrap_text(text: &str, indent: &str) -> String {
    let options = Options::new(WRAP_WIDTH.saturating_sub(indent.len()))
        .initial_indent("")
        .subsequent_indent("");
    wrap(text, options)
        .iter()
        .map(|line| format!("{}{}", indent, line))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Display a path relative to cwd if it's under cwd, otherwise use display_path.
fn display_relative_path(path: &Path, cwd: Option<&Path>) -> String {
    if let Some(cwd) = cwd
        && let Ok(relative) = path.strip_prefix(cwd)
    {
        let rel_str = relative.display().to_string();
        if rel_str.is_empty() {
            return ".".to_string();
        }
        return format!("./{}", rel_str);
    }
    display_path(path)
}

#[cfg(test)]
mod tests {
    use super::format_status;
    use crate::status::SyncStatus;

    #[test]
    fn disables_color_output() {
        let formatted = format_status(SyncStatus::Modified, false);
        assert_eq!(formatted, "modified");
    }
}
