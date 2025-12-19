//! Path expansion and normalization utilities.

use std::path::{MAIN_SEPARATOR, Path, PathBuf};

use path_clean::PathClean;

use crate::error::{Error, Result};

/// Return the default config path for the current platform.
pub fn default_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or(Error::HomeDirMissing)?;
    Ok(home.join(".skills.toml"))
}

/// Expand a config-provided path and resolve it relative to a base directory.
pub fn expand_source_path(raw: &str, base_dir: &Path) -> Result<PathBuf> {
    let expanded = shellexpand::full(raw).map_err(|error| Error::PathExpansion {
        path: raw.to_string(),
        source: error,
    })?;
    let expanded_path = PathBuf::from(expanded.as_ref());
    let resolved = if expanded_path.is_relative() {
        base_dir.join(expanded_path)
    } else {
        expanded_path
    };
    Ok(normalize_path(&resolved))
}

/// Normalize a path for comparisons by cleaning and canonicalizing when possible.
pub fn normalize_path(path: &Path) -> PathBuf {
    match dunce::canonicalize(path) {
        Ok(canonical) => canonical,
        Err(_) => path.clean(),
    }
}

/// Render a path for display, using a tilde prefix for the home directory.
pub fn display_path(path: &Path) -> String {
    if let Some(home) = dirs::home_dir()
        && let Ok(stripped) = path.strip_prefix(&home)
    {
        if stripped.as_os_str().is_empty() {
            return "~".to_string();
        }
        return format!("~{}{}", MAIN_SEPARATOR, stripped.display());
    }
    path.display().to_string()
}
