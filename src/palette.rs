//! Color palette and styling for CLI output.
//!
//! This module defines a consistent visual style for all CLI output.
//! Colors are designed for modern terminals with full color support.

use owo_colors::{OwoColorize, Style};

/// Style for skill names - the primary identifier, visually prominent.
pub fn skill_name() -> Style {
    Style::new().cyan().bold()
}

/// Style for section headings like "Local Skills:" or "Conflicts:".
pub fn heading() -> Style {
    Style::new().white().bold()
}

/// Style for labels like "source:", "path:", "claude:", "codex:".
pub fn label() -> Style {
    Style::new().blue()
}

/// Style for description text - readable but subdued.
pub fn description() -> Style {
    Style::new().dimmed()
}

/// Style for path values.
pub fn path() -> Style {
    Style::new().white()
}

/// Style for tool tags like "[claude]" or "[codex]".
pub fn tool_tag() -> Style {
    Style::new().dimmed()
}

/// Style for synced status.
pub fn status_synced() -> Style {
    Style::new().green()
}

/// Style for modified status.
pub fn status_modified() -> Style {
    Style::new().yellow()
}

/// Style for missing/orphan status.
pub fn status_error() -> Style {
    Style::new().red()
}

/// Style for warning headings.
pub fn warning_heading() -> Style {
    Style::new().yellow().bold()
}

/// Style for warning text.
pub fn warning() -> Style {
    Style::new().yellow()
}

/// Format a skill name with styling.
pub fn fmt_skill_name(name: &str, use_color: bool) -> String {
    if use_color {
        name.style(skill_name()).to_string()
    } else {
        name.to_string()
    }
}

/// Format a section heading with styling.
pub fn fmt_heading(text: &str, use_color: bool) -> String {
    if use_color {
        text.style(heading()).to_string()
    } else {
        text.to_string()
    }
}

/// Format a label with styling.
pub fn fmt_label(text: &str, use_color: bool) -> String {
    if use_color {
        text.style(label()).to_string()
    } else {
        text.to_string()
    }
}

/// Format description text with styling.
pub fn fmt_description(text: &str, use_color: bool) -> String {
    if use_color {
        text.style(description()).to_string()
    } else {
        text.to_string()
    }
}

/// Format a path with styling.
pub fn fmt_path(text: &str, use_color: bool) -> String {
    if use_color {
        text.style(path()).to_string()
    } else {
        text.to_string()
    }
}

/// Format a tool tag with styling.
pub fn fmt_tool_tag(text: &str, use_color: bool) -> String {
    if use_color {
        text.style(tool_tag()).to_string()
    } else {
        text.to_string()
    }
}

/// Format a warning heading with styling.
pub fn fmt_warning_heading(text: &str, use_color: bool) -> String {
    if use_color {
        text.style(warning_heading()).to_string()
    } else {
        text.to_string()
    }
}

/// Format warning text with styling.
pub fn fmt_warning(text: &str, use_color: bool) -> String {
    if use_color {
        text.style(warning()).to_string()
    } else {
        text.to_string()
    }
}
