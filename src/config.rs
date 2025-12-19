//! Configuration loading and validation.

use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::{
    error::{Error, Result},
    paths,
};

/// Parsed configuration for the CLI.
#[derive(Debug, Clone)]
pub struct Config {
    /// Ordered list of configured source directories.
    sources: Vec<PathBuf>,
}

/// Raw config file structure.
#[derive(Debug, Deserialize)]
struct RawConfig {
    /// Ordered list of configured source directories.
    sources: Option<Vec<String>>,
}

impl Config {
    /// Load the default config from disk.
    pub(crate) fn load() -> Result<Self> {
        let path = paths::default_config_path()?;
        Self::load_from(&path)
    }

    /// Load a config file from an explicit path.
    pub(crate) fn load_from(path: &Path) -> Result<Self> {
        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Err(Error::NoSources {
                    config_path: path.to_path_buf(),
                });
            }
            Err(error) => {
                return Err(Error::ConfigRead {
                    path: path.to_path_buf(),
                    source: error,
                });
            }
        };

        let raw: RawConfig = toml::from_str(&contents).map_err(|error| Error::ConfigParse {
            path: path.to_path_buf(),
            source: error,
        })?;

        let Some(raw_sources) = raw.sources else {
            return Err(Error::NoSources {
                config_path: path.to_path_buf(),
            });
        };

        if raw_sources.is_empty() {
            return Err(Error::NoSources {
                config_path: path.to_path_buf(),
            });
        }

        let base_dir = path.parent().unwrap_or(Path::new("."));
        let mut sources = Vec::new();
        for source in raw_sources {
            let expanded = paths::expand_source_path(&source, base_dir)?;
            sources.push(expanded);
        }

        Ok(Self { sources })
    }

    /// Return the configured source directories.
    pub(crate) fn sources(&self) -> &[PathBuf] {
        &self.sources
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::{config::Config, error::Error};

    #[test]
    fn errors_when_sources_missing() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("skills.toml");
        fs::write(&path, "sources = []").expect("write config");

        let error = Config::load_from(&path).expect_err("config should fail");
        assert!(matches!(error, Error::NoSources { .. }));
    }

    #[test]
    fn errors_when_config_missing() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("missing.toml");

        let error = Config::load_from(&path).expect_err("config should fail");
        assert!(matches!(error, Error::NoSources { .. }));
    }
}
