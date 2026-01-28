//! Implementation of the `skills import` command.

use std::{
    env,
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use owo_colors::OwoColorize;
use url::Url;
use zip::{ZipArchive, write::SimpleFileOptions};

use crate::{
    commands::{ColorChoice, init},
    config::Config,
    diagnostics::Diagnostics,
    error::{Error, Result},
    frontmatter::parse_frontmatter,
    paths::{default_config_path, display_path},
    tool::Tool,
};

/// Maximum download size in bytes (10 MB).
const MAX_DOWNLOAD_SIZE: u64 = 10 * 1024 * 1024;

/// Execute the import command.
#[allow(clippy::redundant_clone)]
pub async fn run(
    color: ColorChoice,
    verbose: bool,
    source: String,
    to: Option<String>,
    local: bool,
    force: bool,
    dry_run: bool,
) -> Result<()> {
    init::ensure().await?;
    let diagnostics = Diagnostics::new(verbose);
    let _config = Config::load()?;
    let use_color = color.enabled();

    // Determine source type and get ZIP data
    let (zip_data, source_display) = if is_url(&source) {
        download_source(&source).await?
    } else if is_github_url(&source) {
        download_github(&source).await?
    } else {
        // Local file
        let path = PathBuf::from(&source);
        if !path.exists() {
            return Err(Error::PathMissing { path });
        }
        let data = fs::read(&path).map_err(|e| Error::ZipRead {
            path: path.clone(),
            message: e.to_string(),
        })?;
        (data, source.clone())
    };

    // Parse the ZIP and extract skill info
    let skill_info = parse_zip(&zip_data)?;

    // Determine target locations
    let targets = resolve_targets(&to, local, &skill_info.name)?;

    // Check for existing skills
    for target in &targets {
        if target.exists() && !force {
            return Err(Error::SkillExists {
                name: skill_info.name.clone(),
                path: target.clone(),
            });
        }
    }

    // Print what we're doing
    if use_color {
        println!(
            "{} '{}' from {}",
            if dry_run { "Would import" } else { "Importing" }.bold(),
            skill_info.name.cyan(),
            source_display
        );
    } else {
        println!(
            "{} '{}' from {}",
            if dry_run { "Would import" } else { "Importing" },
            skill_info.name,
            source_display
        );
    }
    println!();

    if dry_run {
        println!("Would extract to:");
        for target in &targets {
            println!("  {}", display_path(target));
        }
        println!();
        println!("Contents:");
        for file in &skill_info.files {
            println!("  - {}", file);
        }
        println!();
        println!("Dry run - no changes made.");
        return Ok(());
    }

    // Extract to each target
    println!("Extracting to:");
    for target in &targets {
        extract_zip(&zip_data, &skill_info.root_dir, target)?;
        println!("  {}", display_path(target));
    }
    println!();

    println!("Contents:");
    for file in &skill_info.files {
        println!("  - {}", file);
    }
    println!();

    println!("Done. Skill is now available.");
    if !local && to.is_none() {
        println!(
            "To manage in your source directory: skills pull {}",
            skill_info.name
        );
    }

    diagnostics.print_skipped_summary();
    Ok(())
}

/// Information extracted from a skill ZIP.
struct SkillInfo {
    /// Skill name from frontmatter.
    name: String,
    /// Root directory in the ZIP.
    root_dir: String,
    /// List of files in the skill.
    files: Vec<String>,
}

/// Check if a string looks like a URL.
fn is_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

/// Check if a string is a GitHub tree URL.
fn is_github_url(s: &str) -> bool {
    s.contains("github.com") && s.contains("/tree/")
}

/// Download from a URL.
async fn download_source(url_str: &str) -> Result<(Vec<u8>, String)> {
    let url = Url::parse(url_str).map_err(|_| Error::InvalidUrl {
        url: url_str.to_string(),
    })?;

    // Reject HTTP
    if url.scheme() == "http" {
        return Err(Error::HttpNotAllowed {
            url: url_str.to_string(),
        });
    }

    let response = reqwest::get(url_str).await.map_err(|e| Error::Download {
        url: url_str.to_string(),
        message: e.to_string(),
    })?;

    // Check content length if available
    if let Some(len) = response.content_length()
        && len > MAX_DOWNLOAD_SIZE
    {
        return Err(Error::FileTooLarge {
            size: len,
            max: MAX_DOWNLOAD_SIZE,
        });
    }

    let bytes = response.bytes().await.map_err(|e| Error::Download {
        url: url_str.to_string(),
        message: e.to_string(),
    })?;

    if bytes.len() as u64 > MAX_DOWNLOAD_SIZE {
        return Err(Error::FileTooLarge {
            size: bytes.len() as u64,
            max: MAX_DOWNLOAD_SIZE,
        });
    }

    Ok((bytes.to_vec(), url_str.to_string()))
}

/// Download a skill directory from GitHub.
async fn download_github(url_str: &str) -> Result<(Vec<u8>, String)> {
    // Parse GitHub URL: https://github.com/owner/repo/tree/ref/path/to/skill
    let url = Url::parse(url_str).map_err(|_| Error::InvalidUrl {
        url: url_str.to_string(),
    })?;

    let path_segments: Vec<&str> = url.path_segments().map_or(vec![], |s| s.collect());

    if path_segments.len() < 4 || path_segments[2] != "tree" {
        return Err(Error::InvalidUrl {
            url: url_str.to_string(),
        });
    }

    let owner = path_segments[0];
    let repo = path_segments[1];
    let git_ref = path_segments[3];
    let skill_path = if path_segments.len() > 4 {
        path_segments[4..].join("/")
    } else {
        String::new()
    };

    // Download the repo ZIP
    let zip_url = format!(
        "https://api.github.com/repos/{}/{}/zipball/{}",
        owner, repo, git_ref
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&zip_url)
        .header("User-Agent", "skills-cli")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| Error::Download {
            url: zip_url.clone(),
            message: e.to_string(),
        })?;

    if !response.status().is_success() {
        return Err(Error::Download {
            url: zip_url,
            message: format!("GitHub API returned {}", response.status()),
        });
    }

    let bytes = response.bytes().await.map_err(|e| Error::Download {
        url: zip_url.clone(),
        message: e.to_string(),
    })?;

    // Extract just the skill subdirectory from the GitHub ZIP
    let extracted = extract_github_subdir(&bytes, &skill_path)?;

    Ok((extracted, url_str.to_string()))
}

/// Extract a subdirectory from a GitHub repo ZIP and repackage it.
fn extract_github_subdir(zip_data: &[u8], subdir: &str) -> Result<Vec<u8>> {
    let cursor = io::Cursor::new(zip_data);
    let mut archive = ZipArchive::new(cursor).map_err(|e| Error::InvalidZip {
        message: e.to_string(),
    })?;

    // GitHub ZIPs have a root directory like "owner-repo-hash/"
    // Find the root prefix
    let root_prefix = archive
        .file_names()
        .next()
        .and_then(|name| name.split('/').next())
        .map(|s| s.to_string())
        .ok_or_else(|| Error::InvalidZip {
            message: "empty ZIP archive".to_string(),
        })?;

    // Build the full path to look for
    let target_prefix = if subdir.is_empty() {
        root_prefix
    } else {
        format!("{}/{}", root_prefix, subdir)
    };

    // Create a new ZIP with just the subdirectory contents
    let mut output = Vec::new();
    {
        let mut writer = zip::ZipWriter::new(io::Cursor::new(&mut output));
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        // Get the skill directory name (last component of subdir)
        let skill_name = subdir.rsplit('/').next().unwrap_or(subdir);
        if skill_name.is_empty() {
            return Err(Error::InvalidZip {
                message: "cannot import repository root, specify a skill path".to_string(),
            });
        }

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| Error::InvalidZip {
                message: e.to_string(),
            })?;

            let name = file.name().to_string();
            if !name.starts_with(&target_prefix) {
                continue;
            }

            // Strip the GitHub prefix and rebuild with just skill name
            let rel_path = name.strip_prefix(&format!("{}/", target_prefix)).unwrap_or(&name);
            if rel_path.is_empty() {
                continue;
            }

            let new_path = format!("{}/{}", skill_name, rel_path);

            if file.is_dir() {
                writer.add_directory(&new_path, options).map_err(|e| Error::InvalidZip {
                    message: e.to_string(),
                })?;
            } else {
                writer.start_file(&new_path, options).map_err(|e| Error::InvalidZip {
                    message: e.to_string(),
                })?;
                let mut contents = Vec::new();
                file.read_to_end(&mut contents).map_err(|e| Error::InvalidZip {
                    message: e.to_string(),
                })?;
                writer.write_all(&contents).map_err(|e| Error::InvalidZip {
                    message: e.to_string(),
                })?;
            }
        }

        writer.finish().map_err(|e| Error::InvalidZip {
            message: e.to_string(),
        })?;
    }

    if output.is_empty() {
        return Err(Error::InvalidZip {
            message: format!("skill path '{}' not found in repository", subdir),
        });
    }

    Ok(output)
}

/// Parse a ZIP archive and extract skill information.
fn parse_zip(data: &[u8]) -> Result<SkillInfo> {
    let cursor = io::Cursor::new(data);
    let mut archive = ZipArchive::new(cursor).map_err(|e| Error::InvalidZip {
        message: e.to_string(),
    })?;

    // Find the root directory
    let root_dir = archive
        .file_names()
        .filter_map(|name| {
            let parts: Vec<&str> = name.split('/').collect();
            if !parts.is_empty() && !parts[0].is_empty() {
                Some(parts[0].to_string())
            } else {
                None
            }
        })
        .next()
        .ok_or_else(|| Error::InvalidZip {
            message: "no root directory found".to_string(),
        })?;

    // Find SKILL.md (case-insensitive)
    let skill_md_path = archive
        .file_names()
        .find(|name| {
            let lower = name.to_lowercase();
            lower == format!("{}/skill.md", root_dir.to_lowercase())
        })
        .map(|s| s.to_string())
        .ok_or_else(|| Error::InvalidZip {
            message: "missing SKILL.md".to_string(),
        })?;

    // Read and parse SKILL.md
    let (contents, frontmatter) = {
        let mut skill_md_file = archive.by_name(&skill_md_path).map_err(|e| Error::InvalidZip {
            message: e.to_string(),
        })?;
        let mut contents = String::new();
        skill_md_file
            .read_to_string(&mut contents)
            .map_err(|e| Error::InvalidZip {
                message: format!("failed to read SKILL.md: {}", e),
            })?;

        let frontmatter = parse_frontmatter(&contents).map_err(|e| Error::InvalidZip {
            message: format!("invalid SKILL.md: {}", e.message),
        })?;
        (contents, frontmatter)
    };
    let _ = contents; // silence unused warning

    // Collect file list
    let files: Vec<String> = archive
        .file_names()
        .filter_map(|name| {
            name.strip_prefix(&format!("{}/", root_dir))
                .filter(|s| !s.is_empty() && !s.ends_with('/'))
                .map(|s| s.to_string())
        })
        .collect();

    Ok(SkillInfo {
        name: frontmatter.name,
        root_dir,
        files,
    })
}

/// Resolve target directories for extraction.
fn resolve_targets(to: &Option<String>, local: bool, skill_name: &str) -> Result<Vec<PathBuf>> {
    if local {
        // Extract to local project directories
        let cwd = env::current_dir().map_err(|_| Error::HomeDirMissing)?;
        let mut paths = Vec::new();
        for tool in Tool::all() {
            paths.push(cwd.join(tool.local_skills_dir()).join(skill_name));
        }
        Ok(paths)
    } else if let Some(target) = to {
        // Check if target is a tool name
        for tool in Tool::all() {
            if target == tool.id() {
                return Ok(vec![tool.skills_dir()?.join(skill_name)]);
            }
        }

        match target.as_str() {
            "source" => {
                // Use first configured source
                let config = Config::load()?;
                let sources = config.sources();
                if sources.is_empty() {
                    return Err(Error::NoSources {
                        config_path: default_config_path()?,
                    });
                }
                Ok(vec![sources[0].join(skill_name)])
            }
            path => {
                // Custom path
                Ok(vec![PathBuf::from(path).join(skill_name)])
            }
        }
    } else {
        // Default: all global directories
        let mut paths = Vec::new();
        for tool in Tool::all() {
            paths.push(tool.skills_dir()?.join(skill_name));
        }
        Ok(paths)
    }
}

/// Extract a ZIP to a target directory.
fn extract_zip(data: &[u8], root_dir: &str, target: &Path) -> Result<()> {
    let cursor = io::Cursor::new(data);
    let mut archive = ZipArchive::new(cursor).map_err(|e| Error::InvalidZip {
        message: e.to_string(),
    })?;

    // Create target directory
    fs::create_dir_all(target).map_err(|e| Error::SkillWrite {
        path: target.to_path_buf(),
        source: e,
    })?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| Error::InvalidZip {
            message: e.to_string(),
        })?;

        let name = file.name().to_string();

        // Strip root directory
        let rel_path = match name.strip_prefix(&format!("{}/", root_dir)) {
            Some(p) if !p.is_empty() => p,
            _ => continue,
        };

        // Security check: no path traversal
        if rel_path.contains("..") {
            return Err(Error::InvalidZip {
                message: format!("path traversal detected: {}", rel_path),
            });
        }

        let out_path = target.join(rel_path);

        if file.is_dir() {
            fs::create_dir_all(&out_path).map_err(|e| Error::SkillWrite {
                path: out_path.clone(),
                source: e,
            })?;
        } else {
            // Ensure parent directory exists
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent).map_err(|e| Error::SkillWrite {
                    path: parent.to_path_buf(),
                    source: e,
                })?;
            }

            let mut outfile = File::create(&out_path).map_err(|e| Error::SkillWrite {
                path: out_path.clone(),
                source: e,
            })?;

            io::copy(&mut file, &mut outfile).map_err(|e| Error::SkillWrite {
                path: out_path.clone(),
                source: e,
            })?;
        }
    }

    Ok(())
}
