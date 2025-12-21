//! Implementation of the `skills pack` command.

use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use owo_colors::OwoColorize;
use walkdir::WalkDir;
use zip::{ZipWriter, write::SimpleFileOptions};

use crate::{
    catalog::Catalog,
    commands::{ColorChoice, init},
    config::Config,
    diagnostics::Diagnostics,
    error::{Error, Result},
    paths::display_path,
};

/// Result of packing a single skill.
struct PackResult {
    /// Skill name.
    name: String,
    /// Output path.
    path: PathBuf,
    /// Size in bytes.
    size: u64,
    /// Files included.
    files: Vec<String>,
}

/// Execute the pack command for specific skills.
#[allow(clippy::too_many_arguments)]
pub async fn run(
    color: ColorChoice,
    verbose: bool,
    skill_names: Vec<String>,
    all: bool,
    output: Option<PathBuf>,
    local: bool,
    dry_run: bool,
    force: bool,
) -> Result<()> {
    init::ensure().await?;
    let mut diagnostics = Diagnostics::new(verbose);
    let config = Config::load()?;
    let catalog = Catalog::load(&config, &mut diagnostics);
    let use_color = color.enabled();

    let output_dir = output.unwrap_or_else(|| PathBuf::from("."));

    // Ensure output directory exists
    if !output_dir.exists() {
        if dry_run {
            println!("Would create directory: {}", display_path(&output_dir));
        } else {
            fs::create_dir_all(&output_dir).map_err(|e| Error::SkillWrite {
                path: output_dir.clone(),
                source: e,
            })?;
        }
    }

    // If --all or no skills specified, pack all skills
    if all || skill_names.is_empty() {
        return pack_all(&catalog, &output_dir, dry_run, force, use_color, local, &mut diagnostics);
    }

    if skill_names.len() == 1 {
        // Single skill - use detailed output
        pack_single(&catalog, &skill_names[0], &output_dir, dry_run, force, use_color, local)
    } else {
        // Multiple skills - use summary output
        pack_multiple(&catalog, &skill_names, &output_dir, dry_run, force, use_color, local, &mut diagnostics)
    }
}

/// Execute the pack-all command.
pub async fn run_all(
    color: ColorChoice,
    verbose: bool,
    output: PathBuf,
    local: bool,
    dry_run: bool,
    force: bool,
) -> Result<()> {
    init::ensure().await?;
    let mut diagnostics = Diagnostics::new(verbose);
    let config = Config::load()?;
    let catalog = Catalog::load(&config, &mut diagnostics);
    let use_color = color.enabled();

    pack_all(&catalog, &output, dry_run, force, use_color, local, &mut diagnostics)
}

/// Pack a single skill with detailed output.
fn pack_single(
    catalog: &Catalog,
    name: &str,
    output_dir: &Path,
    dry_run: bool,
    force: bool,
    use_color: bool,
    local: bool,
) -> Result<()> {
    // Find the skill
    let skill_dir = if local {
        find_local_skill(catalog, name)?
    } else {
        find_source_skill(catalog, name)?
    };

    // Determine output path
    let output_path = output_dir.join(format!("{}.zip", name));

    // Check if output exists
    if output_path.exists() && !force {
        return Err(Error::PathExists {
            path: output_path,
        });
    }

    if dry_run {
        println!(
            "{} '{}' from {}",
            "Would pack".bold(),
            name,
            display_path(&skill_dir)
        );
        let files = collect_files(&skill_dir)?;
        println!("\nFiles:");
        for file in &files {
            println!("  - {}", file);
        }
        println!("\nDry run - no changes made.");
        return Ok(());
    }

    // Pack the skill
    let result = pack_skill(name, &skill_dir, &output_path)?;

    // Print result
    if use_color {
        println!(
            "{} '{}' from {}",
            "Packing".bold(),
            result.name.cyan(),
            display_path(&skill_dir)
        );
    } else {
        println!("Packing '{}' from {}", result.name, display_path(&skill_dir));
    }
    println!();
    println!(
        "Created: {} ({} bytes)",
        display_path(&result.path),
        result.size
    );
    for file in &result.files {
        println!("  - {}", file);
    }
    println!();
    println!(
        "Share this file or import with: skills import {}",
        result.path.file_name().unwrap_or_default().to_string_lossy()
    );

    Ok(())
}

/// Pack multiple named skills with summary output.
#[allow(clippy::too_many_arguments)]
fn pack_multiple(
    catalog: &Catalog,
    names: &[String],
    output_dir: &Path,
    dry_run: bool,
    force: bool,
    use_color: bool,
    local: bool,
    diagnostics: &mut Diagnostics,
) -> Result<()> {
    println!(
        "Packing {} skills{}...",
        names.len(),
        if dry_run { " (dry run)" } else { "" }
    );
    println!();

    let mut success_count = 0;
    let mut skip_count = 0;

    for name in names {
        let skill_dir = if local {
            match find_local_skill(catalog, name) {
                Ok(dir) => dir,
                Err(e) => {
                    diagnostics.warn(format!("Skill '{}': {}", name, e));
                    if use_color {
                        println!("  {} {} (not found)", "✗".red(), name);
                    } else {
                        println!("  ✗ {} (not found)", name);
                    }
                    skip_count += 1;
                    continue;
                }
            }
        } else {
            match find_source_skill(catalog, name) {
                Ok(dir) => dir,
                Err(e) => {
                    diagnostics.warn(format!("Skill '{}': {}", name, e));
                    if use_color {
                        println!("  {} {} (not found)", "✗".red(), name);
                    } else {
                        println!("  ✗ {} (not found)", name);
                    }
                    skip_count += 1;
                    continue;
                }
            }
        };

        let output_path = output_dir.join(format!("{}.zip", name));

        // Check if output exists
        if output_path.exists() && !force {
            if use_color {
                println!("  {} {} (already exists)", "✗".red(), name);
            } else {
                println!("  ✗ {} (already exists)", name);
            }
            skip_count += 1;
            continue;
        }

        if dry_run {
            if use_color {
                println!("  {} {}.zip", "✓".green(), name);
            } else {
                println!("  ✓ {}.zip", name);
            }
            success_count += 1;
            continue;
        }

        match pack_skill(name, &skill_dir, &output_path) {
            Ok(result) => {
                if use_color {
                    println!("  {} {}.zip ({} bytes)", "✓".green(), name, result.size);
                } else {
                    println!("  ✓ {}.zip ({} bytes)", name, result.size);
                }
                success_count += 1;
            }
            Err(e) => {
                diagnostics.warn(format!("Failed to pack '{}': {}", name, e));
                if use_color {
                    println!("  {} {} ({})", "✗".red(), name, e);
                } else {
                    println!("  ✗ {} ({})", name, e);
                }
                skip_count += 1;
            }
        }
    }

    println!();
    if dry_run {
        println!(
            "Would create {} skill archives in {}",
            success_count,
            display_path(output_dir)
        );
    } else {
        println!(
            "Created {} skill archives in {}",
            success_count,
            display_path(output_dir)
        );
    }
    if skip_count > 0 {
        println!("Skipped {} skills", skip_count);
    }

    diagnostics.print_skipped_summary();
    Ok(())
}

/// Pack all skills from sources.
fn pack_all(
    catalog: &Catalog,
    output_dir: &Path,
    dry_run: bool,
    force: bool,
    use_color: bool,
    local: bool,
    diagnostics: &mut Diagnostics,
) -> Result<()> {
    // Ensure output directory exists
    if !output_dir.exists() {
        if dry_run {
            println!("Would create directory: {}", display_path(output_dir));
        } else {
            fs::create_dir_all(output_dir).map_err(|e| Error::SkillWrite {
                path: output_dir.to_path_buf(),
                source: e,
            })?;
        }
    }

    let skills: Vec<(&String, PathBuf)> = if local {
        // Collect local skills
        catalog
            .local
            .values()
            .flat_map(|skills| skills.iter())
            .map(|(name, skill)| (name, skill.skill_dir.clone()))
            .collect()
    } else {
        // Collect source skills
        catalog
            .sources
            .iter()
            .map(|(name, skill)| (name, skill.skill_dir.clone()))
            .collect()
    };

    if skills.is_empty() {
        println!("No skills found to pack.");
        return Ok(());
    }

    println!(
        "Packing {} skills{}...",
        if local { "local" } else { "all" },
        if dry_run { " (dry run)" } else { "" }
    );
    println!();

    let mut success_count = 0;
    let mut skip_count = 0;

    for (name, skill_dir) in skills {
        let output_path = output_dir.join(format!("{}.zip", name));

        // Check if output exists
        if output_path.exists() && !force {
            if use_color {
                println!("  {} {} (already exists)", "✗".red(), name);
            } else {
                println!("  ✗ {} (already exists)", name);
            }
            skip_count += 1;
            continue;
        }

        if dry_run {
            if use_color {
                println!("  {} {}.zip", "✓".green(), name);
            } else {
                println!("  ✓ {}.zip", name);
            }
            success_count += 1;
            continue;
        }

        match pack_skill(name, &skill_dir, &output_path) {
            Ok(result) => {
                if use_color {
                    println!("  {} {}.zip ({} bytes)", "✓".green(), name, result.size);
                } else {
                    println!("  ✓ {}.zip ({} bytes)", name, result.size);
                }
                success_count += 1;
            }
            Err(e) => {
                diagnostics.warn(format!("Failed to pack '{}': {}", name, e));
                if use_color {
                    println!("  {} {} ({})", "✗".red(), name, e);
                } else {
                    println!("  ✗ {} ({})", name, e);
                }
                skip_count += 1;
            }
        }
    }

    println!();
    if dry_run {
        println!(
            "Would create {} skill archives in {}",
            success_count,
            display_path(output_dir)
        );
    } else {
        println!(
            "Created {} skill archives in {}",
            success_count,
            display_path(output_dir)
        );
    }
    if skip_count > 0 {
        println!("Skipped {} skills", skip_count);
    }

    diagnostics.print_skipped_summary();
    Ok(())
}

/// Find a source skill by name.
fn find_source_skill(catalog: &Catalog, name: &str) -> Result<PathBuf> {
    catalog
        .sources
        .get(name)
        .map(|s| s.skill_dir.clone())
        .ok_or_else(|| Error::SkillNotFound {
            name: name.to_string(),
        })
}

/// Find a local skill by name.
fn find_local_skill(catalog: &Catalog, name: &str) -> Result<PathBuf> {
    for skills in catalog.local.values() {
        if let Some(skill) = skills.get(name) {
            return Ok(skill.skill_dir.clone());
        }
    }
    Err(Error::LocalSkillNotFound {
        name: name.to_string(),
    })
}

/// Collect relative file paths in a directory.
fn collect_files(dir: &Path) -> Result<Vec<String>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(dir).min_depth(1) {
        let entry = entry.map_err(|e| Error::SkillRead {
            path: dir.to_path_buf(),
            source: e
                .into_io_error()
                .unwrap_or_else(|| io::Error::other("walkdir error")),
        })?;
        if entry.file_type().is_file()
            && let Ok(rel) = entry.path().strip_prefix(dir)
        {
            files.push(rel.display().to_string());
        }
    }
    files.sort();
    Ok(files)
}

/// Pack a skill directory into a ZIP file.
fn pack_skill(name: &str, skill_dir: &Path, output_path: &Path) -> Result<PackResult> {
    let file = File::create(output_path).map_err(|e| Error::ZipCreate {
        path: output_path.to_path_buf(),
        message: e.to_string(),
    })?;

    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);

    let mut files = Vec::new();

    for entry in WalkDir::new(skill_dir).min_depth(1) {
        let entry = entry.map_err(|e| Error::ZipCreate {
            path: output_path.to_path_buf(),
            message: e.to_string(),
        })?;

        let path = entry.path();
        let rel_path = path.strip_prefix(skill_dir).map_err(|_| Error::ZipCreate {
            path: output_path.to_path_buf(),
            message: "failed to compute relative path".to_string(),
        })?;

        // Build archive path with skill name as root directory
        let archive_path = format!("{}/{}", name, rel_path.display());

        if entry.file_type().is_dir() {
            zip.add_directory(&archive_path, options).map_err(|e| Error::ZipCreate {
                path: output_path.to_path_buf(),
                message: e.to_string(),
            })?;
        } else if entry.file_type().is_file() {
            zip.start_file(&archive_path, options).map_err(|e| Error::ZipCreate {
                path: output_path.to_path_buf(),
                message: e.to_string(),
            })?;

            let mut f = File::open(path).map_err(|e| Error::SkillRead {
                path: path.to_path_buf(),
                source: e,
            })?;
            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer).map_err(|e| Error::SkillRead {
                path: path.to_path_buf(),
                source: e,
            })?;
            zip.write_all(&buffer).map_err(|e| Error::ZipCreate {
                path: output_path.to_path_buf(),
                message: e.to_string(),
            })?;

            files.push(rel_path.display().to_string());
        }
    }

    zip.finish().map_err(|e| Error::ZipCreate {
        path: output_path.to_path_buf(),
        message: e.to_string(),
    })?;

    let size = fs::metadata(output_path)
        .map(|m| m.len())
        .unwrap_or(0);

    files.sort();

    Ok(PackResult {
        name: name.to_string(),
        path: output_path.to_path_buf(),
        size,
        files,
    })
}
