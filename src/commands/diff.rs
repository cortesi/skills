//! Implementation of the `skills diff` command.

use owo_colors::OwoColorize;

use crate::{
    catalog::Catalog,
    commands::{ColorChoice, init},
    config::Config,
    diagnostics::Diagnostics,
    diff::{colorize_diff, resolve_pager, unified_diff, write_output},
    error::{Error, Result},
    paths::display_path,
    skill::render_template,
    status::{SyncStatus, normalize_line_endings},
    tool::Tool,
};

/// Execute the diff command.
pub async fn run(
    color: ColorChoice,
    verbose: bool,
    skill: Option<String>,
    pager: Option<String>,
) -> Result<()> {
    init::ensure().await?;
    let mut diagnostics = Diagnostics::new(verbose);
    let config = Config::load()?;
    let catalog = Catalog::load(&config, &mut diagnostics);

    let mut names = collect_names(&catalog, skill.as_deref())?;
    names.sort_by_key(|left| left.to_lowercase());

    let mut output = String::new();
    let use_color = color.enabled();

    for name in names {
        let mut section = String::new();
        if !output.is_empty() {
            section.push('\n');
        }
        section.push_str(&format_header(&name));
        section.push('\n');

        let source = catalog.sources.get(&name);
        let mut skip = false;

        for tool in Tool::all() {
            let tool_map = catalog.tools.get(&tool);
            let tool_skill = tool_map.and_then(|skills| skills.get(&name));
            let mut rendered = None;

            let status = match (source, tool_skill) {
                (Some(source), Some(tool_skill)) => {
                    let rendered_value = match render_template(&source.contents, tool) {
                        Ok(rendered) => rendered,
                        Err(error) => {
                            diagnostics.warn_skipped(&source.skill_path, error);
                            skip = true;
                            break;
                        }
                    };
                    let synced = normalize_line_endings(&rendered_value)
                        == normalize_line_endings(&tool_skill.contents);
                    rendered = Some(rendered_value);
                    if synced {
                        SyncStatus::Synced
                    } else {
                        SyncStatus::Modified
                    }
                }
                (Some(_), None) => SyncStatus::Missing,
                (None, Some(_)) => SyncStatus::Orphan,
                (None, None) => continue,
            };

            section.push_str(&format_tool_status(tool, status, use_color));
            section.push('\n');

            if status == SyncStatus::Modified
                && let (Some(source), Some(tool_skill), Some(rendered)) =
                    (source, tool_skill, rendered.as_ref())
            {
                let diff_text = unified_diff(
                    &format!("source: {}", display_path(&source.skill_path)),
                    &format!("tool: {}", display_path(&tool_skill.skill_path)),
                    rendered,
                    &tool_skill.contents,
                );
                let diff_text = colorize_diff(&diff_text, use_color);
                section.push_str(&diff_text);
                if !diff_text.ends_with('\n') {
                    section.push('\n');
                }
            }
        }

        if skip {
            continue;
        }

        output.push_str(&section);
    }

    diagnostics.print_skipped_summary();
    let pager = resolve_pager(pager.as_deref());
    write_output(&output, pager.as_deref())?;
    Ok(())
}

/// Collect skill names for diffing.
fn collect_names(catalog: &Catalog, skill: Option<&str>) -> Result<Vec<String>> {
    if let Some(skill) = skill {
        if catalog.sources.contains_key(skill)
            || catalog
                .tools
                .values()
                .any(|skills| skills.contains_key(skill))
        {
            return Ok(vec![skill.to_string()]);
        }
        return Err(Error::SkillNotFound {
            name: skill.to_string(),
        });
    }

    let mut names = Vec::new();
    for name in catalog.sources.keys() {
        names.push(name.clone());
    }
    for tool_map in catalog.tools.values() {
        for name in tool_map.keys() {
            if !names.contains(name) {
                names.push(name.clone());
            }
        }
    }

    Ok(names)
}

/// Format a tool status line for diff output.
fn format_tool_status(tool: Tool, status: SyncStatus, color: bool) -> String {
    let label = match status {
        SyncStatus::Synced => "synced",
        SyncStatus::Modified => "modified",
        SyncStatus::Missing => "missing",
        SyncStatus::Orphan => "orphan",
    };

    let status_text = if !color {
        label.to_string()
    } else {
        match status {
            SyncStatus::Synced => label.green().to_string(),
            SyncStatus::Modified => label.yellow().to_string(),
            SyncStatus::Missing => label.red().to_string(),
            SyncStatus::Orphan => label.red().to_string(),
        }
    };

    format!("{}: {}", tool.display_name(), status_text)
}

/// Format a header for diff output.
fn format_header(name: &str) -> String {
    let mut line = String::new();
    line.push_str("=== ");
    line.push_str(name);
    line.push(' ');
    line.push_str("===");
    line
}
