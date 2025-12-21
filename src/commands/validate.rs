//! Implementation of the `skills validate` command.

use owo_colors::OwoColorize;

use crate::{
    catalog::Catalog,
    commands::{ColorChoice, init},
    config::Config,
    diagnostics::Diagnostics,
    error::Result,
    frontmatter::parse_frontmatter,
    skill::render_template,
    tool::Tool,
};

/// Execute the validate command.
pub async fn run(
    color: ColorChoice,
    verbose: bool,
    skill_name: Option<String>,
) -> Result<()> {
    init::ensure().await?;
    let mut diagnostics = Diagnostics::new(verbose);
    let config = Config::load()?;
    let catalog = Catalog::load(&config, &mut diagnostics);
    let use_color = color.enabled();

    // Collect skills to validate
    let skills_to_validate: Vec<_> = if let Some(name) = skill_name {
        if let Some(skill) = catalog.sources.get(&name) {
            vec![(name, skill)]
        } else {
            println!("Skill '{}' not found in sources.", name);
            return Ok(());
        }
    } else {
        catalog.sources.iter().map(|(n, s)| (n.clone(), s)).collect()
    };

    if skills_to_validate.is_empty() {
        println!("No skills to validate.");
        return Ok(());
    }

    let mut valid_count = 0;
    let mut invalid_count = 0;

    for (name, skill) in skills_to_validate {
        let mut errors = Vec::new();

        // Check 1: Frontmatter parsing
        match parse_frontmatter(&skill.contents) {
            Ok(fm) => {
                // Check name matches directory
                if fm.name != name {
                    errors.push(format!(
                        "frontmatter name '{}' does not match directory name '{}'",
                        fm.name, name
                    ));
                }
                // Check description exists
                if fm.description.is_empty() {
                    errors.push("description is empty".to_string());
                }
            }
            Err(e) => {
                errors.push(format!("frontmatter: {}", e.message));
            }
        }

        // Check 2: Template rendering for all tools
        for tool in Tool::all() {
            if let Err(e) = render_template(&skill.contents, tool) {
                errors.push(format!("template ({} render): {}", tool.id(), e));
            }
        }

        // Print result
        if errors.is_empty() {
            valid_count += 1;
            if use_color {
                println!("{} {}", "✓".green(), name);
            } else {
                println!("✓ {}", name);
            }
        } else {
            invalid_count += 1;
            if use_color {
                println!("{} {}", "✗".red(), name);
            } else {
                println!("✗ {}", name);
            }
            for error in errors {
                println!("    - {}", error);
            }
        }
    }

    println!();
    println!(
        "{} valid, {} invalid",
        valid_count,
        invalid_count
    );

    Ok(())
}
