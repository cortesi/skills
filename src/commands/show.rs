//! Implementation of the `skills show` command.

use std::fs;

use owo_colors::OwoColorize;

use crate::{
    catalog::Catalog,
    commands::{ColorChoice, init},
    config::Config,
    diagnostics::Diagnostics,
    diff::{resolve_pager, write_output},
    error::{Error, Result},
    skill::SKILL_FILE_NAME,
};

/// Execute the show command.
pub async fn run(
    color: ColorChoice,
    verbose: bool,
    skill: String,
    pager: Option<String>,
) -> Result<()> {
    init::ensure().await?;
    let mut diagnostics = Diagnostics::new(verbose);
    let config = Config::load()?;
    let catalog = Catalog::load(&config, &mut diagnostics);
    let use_color = color.enabled();

    // Look for the skill in sources first, then tools, then local
    let contents = find_skill_contents(&catalog, &skill)?;

    let output = if use_color {
        highlight_markdown(&contents)
    } else {
        contents
    };

    let pager = resolve_pager(pager.as_deref());
    write_output(&output, pager.as_deref())?;

    Ok(())
}

/// Find skill contents by name, checking sources, tools, and local skills.
fn find_skill_contents(catalog: &Catalog, name: &str) -> Result<String> {
    // Check source skills
    if let Some(skill) = catalog.sources.get(name) {
        return Ok(skill.contents.clone());
    }

    // Check tool skills (any tool)
    for tool_skills in catalog.tools.values() {
        if let Some(skill) = tool_skills.get(name) {
            return Ok(skill.contents.clone());
        }
    }

    // Check local skills
    for local_skills in catalog.local.values() {
        if let Some(skill) = local_skills.get(name) {
            let skill_path = skill.skill_dir.join(SKILL_FILE_NAME);
            return fs::read_to_string(&skill_path).map_err(|e| Error::SkillRead {
                path: skill_path,
                source: e,
            });
        }
    }

    Err(Error::SkillNotFound {
        name: name.to_string(),
    })
}

/// Highlight markdown content with syntax coloring.
fn highlight_markdown(contents: &str) -> String {
    let mut output = String::new();
    let mut in_code_block = false;
    let mut in_frontmatter = false;
    let mut frontmatter_count = 0;

    for line in contents.lines() {
        // Handle YAML frontmatter
        if line == "---" {
            frontmatter_count += 1;
            if frontmatter_count == 1 {
                in_frontmatter = true;
                output.push_str(&line.dimmed().to_string());
                output.push('\n');
                continue;
            } else if frontmatter_count == 2 {
                in_frontmatter = false;
                output.push_str(&line.dimmed().to_string());
                output.push('\n');
                continue;
            }
        }

        if in_frontmatter {
            output.push_str(&line.dimmed().to_string());
            output.push('\n');
            continue;
        }

        // Handle code blocks
        if line.starts_with("```") {
            in_code_block = !in_code_block;
            output.push_str(&line.cyan().to_string());
            output.push('\n');
            continue;
        }

        if in_code_block {
            output.push_str(&line.cyan().to_string());
            output.push('\n');
            continue;
        }

        // Handle headers
        if line.starts_with('#') {
            output.push_str(&line.bold().white().to_string());
            output.push('\n');
            continue;
        }

        // Handle bullet points
        if line.starts_with("- ") || line.starts_with("* ") {
            let (bullet, rest) = line.split_at(2);
            output.push_str(&bullet.blue().to_string());
            output.push_str(&format_inline(rest));
            output.push('\n');
            continue;
        }

        // Handle numbered lists
        if let Some(pos) = line.find(". ") {
            let prefix = &line[..pos];
            if prefix.chars().all(|c| c.is_ascii_digit()) {
                let number_part: String = line[..=pos].to_string();
                output.push_str(&number_part.blue().to_string());
                output.push_str(&format_inline(&line[pos + 2..]));
                output.push('\n');
                continue;
            }
        }

        // Handle blockquotes
        if line.starts_with('>') {
            output.push_str(&line.yellow().to_string());
            output.push('\n');
            continue;
        }

        // Regular text with inline formatting
        output.push_str(&format_inline(line));
        output.push('\n');
    }

    // Remove trailing newline if original didn't have one
    if !contents.ends_with('\n') && output.ends_with('\n') {
        output.pop();
    }

    output
}

/// Format a line with inline markdown formatting (bold, italic, code, links).
fn format_inline(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut output = String::new();
    let mut i = 0;

    while i < chars.len() {
        // Handle inline code
        if chars[i] == '`'
            && let Some(end) = find_closing(&chars, i + 1, '`')
        {
            let code: String = chars[i + 1..end].iter().collect();
            output.push_str(&format!("`{code}`").cyan().to_string());
            i = end + 1;
            continue;
        }

        // Handle bold (**text**)
        if i + 1 < chars.len()
            && chars[i] == '*'
            && chars[i + 1] == '*'
            && let Some(end) = find_double_closing(&chars, i + 2, '*')
        {
            let bold: String = chars[i + 2..end].iter().collect();
            output.push_str(&bold.bold().to_string());
            i = end + 2;
            continue;
        }

        // Handle italic (*text*)
        if chars[i] == '*'
            && let Some(end) = find_closing(&chars, i + 1, '*')
        {
            let italic: String = chars[i + 1..end].iter().collect();
            output.push_str(&italic.italic().to_string());
            i = end + 1;
            continue;
        }

        // Handle links [text](url)
        if chars[i] == '['
            && let Some(bracket_end) = find_closing(&chars, i + 1, ']')
            && bracket_end + 1 < chars.len()
            && chars[bracket_end + 1] == '('
            && let Some(paren_end) = find_closing(&chars, bracket_end + 2, ')')
        {
            let link_text: String = chars[i + 1..bracket_end].iter().collect();
            let url: String = chars[bracket_end + 2..paren_end].iter().collect();
            output.push_str(&link_text.blue().underline().to_string());
            output.push_str(&format!(" ({url})").dimmed().to_string());
            i = paren_end + 1;
            continue;
        }

        output.push(chars[i]);
        i += 1;
    }

    output
}

/// Find closing delimiter in char slice.
fn find_closing(chars: &[char], start: usize, delim: char) -> Option<usize> {
    for (i, &c) in chars.iter().enumerate().skip(start) {
        if c == delim {
            return Some(i);
        }
    }
    None
}

/// Find closing double delimiter (like **) in char slice.
fn find_double_closing(chars: &[char], start: usize, delim: char) -> Option<usize> {
    let mut i = start;
    while i + 1 < chars.len() {
        if chars[i] == delim && chars[i + 1] == delim {
            return Some(i);
        }
        i += 1;
    }
    None
}
