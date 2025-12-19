//! Implementation of the `skills list` command.

use owo_colors::OwoColorize;

use crate::{
    catalog::Catalog,
    commands::{ColorChoice, init},
    config::Config,
    diagnostics::Diagnostics,
    error::Result,
    paths::display_path,
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

    for entry in entries {
        let source_path = catalog
            .sources
            .get(&entry.name)
            .map(|skill| display_path(&skill.source_root))
            .unwrap_or_else(|| "-".to_string());

        let claude = format_status(status_for_tool(&entry, Tool::Claude), use_color);
        let codex = format_status(status_for_tool(&entry, Tool::Codex), use_color);

        println!("{}", entry.name);
        println!("  source: {}", source_path);
        println!("  claude: {:<9} codex: {:<9}", claude, codex);
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
