//! Implementation of the `skills render` command.

use crate::{
    catalog::Catalog,
    commands::{ColorChoice, init},
    config::Config,
    diagnostics::Diagnostics,
    error::{Error, Result},
    skill::render_template,
    tool::ToolFilter,
};

/// Execute the render command.
pub async fn run(
    _color: ColorChoice,
    verbose: bool,
    skill_name: String,
    tool_filter: ToolFilter,
) -> Result<()> {
    init::ensure().await?;
    let mut diagnostics = Diagnostics::new(verbose);
    let config = Config::load()?;
    let catalog = Catalog::load(&config, &mut diagnostics);

    // Find the skill in sources
    let source = catalog.sources.get(&skill_name).ok_or_else(|| Error::SkillNotFound {
        name: skill_name.clone(),
    })?;

    // Determine which tool(s) to render for
    let tools = tool_filter.to_tools();
    let multi = tools.len() > 1;

    for tool in &tools {
        // Render the template for this tool
        let rendered = render_template(&source.contents, *tool)
            .map_err(|e| Error::TemplateRender { message: e })?;

        if multi {
            println!("=== {} ===", tool.display_name());
        }
        print!("{}", rendered);
        if !rendered.ends_with('\n') {
            println!();
        }
        if multi {
            println!();
        }
    }

    Ok(())
}
