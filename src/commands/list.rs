//! Implementation of the `skills list` command.

use std::{collections::HashSet, env, path::Path};

use textwrap::{Options, wrap};

use crate::{
    catalog::Catalog,
    commands::{ColorChoice, init},
    config::Config,
    diagnostics::Diagnostics,
    error::Result,
    palette::{
        fmt_description, fmt_heading, fmt_label, fmt_path, fmt_skill_name, fmt_tool_tag,
        fmt_warning, fmt_warning_heading, status_error, status_modified, status_synced,
    },
    paths::display_path,
    skill::LocalSkill,
    status::{SkillEntry, SyncStatus, build_entries},
    tool::Tool,
};

/// Indent for subordinate information.
const INDENT: &str = "    ";
/// Double indent for nested subordinate information.
const INDENT2: &str = "        ";

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

        println!("{}", fmt_skill_name(&entry.name, use_color));
        println!(
            "{}{} {}",
            INDENT,
            fmt_label("source:", use_color),
            fmt_path(&source_path, use_color)
        );

        let mut tool_output = String::new();
        for tool in Tool::all() {
            let status = format_status(status_for_tool(entry, tool), use_color);
            let label = format!("{}:", tool.id());
            use std::fmt::Write;
            let _ = write!(
                &mut tool_output,
                "{} {:<9} ",
                fmt_label(&label, use_color),
                status
            );
        }
        println!("{}{}", INDENT, tool_output.trim_end());

        println!("{}", wrap_styled(description, INDENT, use_color));
    }

    // Collect and print local skills
    let local_skills = collect_local_skills(&catalog);
    let cwd = env::current_dir().ok();
    if !local_skills.is_empty() {
        if !entries.is_empty() {
            println!();
        }
        println!("{}", fmt_heading("Local Skills:", use_color));
        for skill in &local_skills {
            let tool_label = format!("[{}]", skill.tool.id());
            let path_display = display_relative_path(&skill.skill_dir, cwd.as_deref());
            println!(
                "{}{} {}",
                INDENT,
                fmt_skill_name(&skill.name, use_color),
                fmt_tool_tag(&tool_label, use_color)
            );
            println!("{}", wrap_styled(&skill.description, INDENT2, use_color));
            println!(
                "{}{} {}",
                INDENT2,
                fmt_label("path:", use_color),
                fmt_path(&path_display, use_color)
            );
        }
    }

    // Print conflicts between local and global skills
    let conflicts = find_conflicts(&catalog);
    if !conflicts.is_empty() {
        println!();
        println!("{}", fmt_warning_heading("Conflicts:", use_color));
        for (name, tool) in &conflicts {
            let warning = format!(
                "{}⚠ '{}' exists locally and in {} global skills",
                INDENT, name, tool.id()
            );
            println!("{}", fmt_warning(&warning, use_color));
            println!("{}Local takes precedence in this project", INDENT2);
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
fn format_status(status: SyncStatus, use_color: bool) -> String {
    use owo_colors::OwoColorize;

    let label = match status {
        SyncStatus::Synced => "synced",
        SyncStatus::Modified => "modified",
        SyncStatus::Missing => "missing",
        SyncStatus::Orphan => "orphan",
    };

    if !use_color {
        return label.to_string();
    }

    match status {
        SyncStatus::Synced => label.style(status_synced()).to_string(),
        SyncStatus::Modified => label.style(status_modified()).to_string(),
        SyncStatus::Missing => label.style(status_error()).to_string(),
        SyncStatus::Orphan => label.style(status_error()).to_string(),
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

/// Wrap text to a given width with an indent prefix and optional styling.
fn wrap_styled(text: &str, indent: &str, use_color: bool) -> String {
    let options = Options::new(WRAP_WIDTH.saturating_sub(indent.len()))
        .initial_indent("")
        .subsequent_indent("");
    wrap(text, options)
        .iter()
        .map(|line| format!("{}{}", indent, fmt_description(line, use_color)))
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
