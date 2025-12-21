//! Error types for the skills CLI.

use std::{
    env::VarError,
    io,
    path::PathBuf,
    process::{ExitCode, ExitStatus},
    result::Result as StdResult,
};

use thiserror::Error;
use toml::{de::Error as TomlError, ser::Error as TomlSerError};

/// Result type for skills operations.
pub type Result<T> = StdResult<T, Error>;

/// Errors that can occur while running the CLI.
#[derive(Debug, Error)]
pub enum Error {
    /// No configured sources were found in the config file.
    #[error("No sources configured; edit {config_path} to add at least one source.")]
    NoSources {
        /// Path to the config file.
        config_path: PathBuf,
    },
    /// The configuration file could not be read.
    #[error("Failed to read config at {path}: {source}")]
    ConfigRead {
        /// Path to the config file.
        path: PathBuf,
        /// Underlying IO error.
        source: io::Error,
    },
    /// The configuration file could not be parsed.
    #[error("Failed to parse config at {path}: {source}")]
    ConfigParse {
        /// Path to the config file.
        path: PathBuf,
        /// Underlying parse error.
        source: TomlError,
    },
    /// The configuration file could not be serialized.
    #[error("Failed to serialize config: {source}")]
    ConfigSerialize {
        /// Underlying serialization error.
        source: TomlSerError,
    },
    /// The configuration file could not be written.
    #[error("Failed to write config at {path}: {source}")]
    ConfigWrite {
        /// Path to the config file.
        path: PathBuf,
        /// Underlying IO error.
        source: io::Error,
    },
    /// Home directory resolution failed.
    #[error("Failed to resolve the home directory.")]
    HomeDirMissing,
    /// A configured path could not be expanded.
    #[error("Invalid path in config: {path}: {source}")]
    PathExpansion {
        /// Input path that failed to expand.
        path: String,
        /// Underlying expansion error.
        source: shellexpand::LookupError<VarError>,
    },
    /// A configured path was not valid Unicode for expansion.
    #[error("Invalid path in config: {path}")]
    PathNotUnicode {
        /// Path that could not be represented as UTF-8.
        path: PathBuf,
    },
    /// A tool pager was specified but could not be parsed.
    #[error("Invalid pager command: {message}")]
    PagerParse {
        /// Error message describing the parse failure.
        message: String,
    },
    /// A pager command could not be spawned.
    #[error("Failed to run pager `{pager}`: {source}")]
    PagerSpawn {
        /// Pager command that failed to spawn.
        pager: String,
        /// Underlying spawn error.
        source: io::Error,
    },
    /// A pager process exited with a non-zero status.
    #[error("Pager `{pager}` exited with status {status}")]
    PagerStatus {
        /// Pager command that exited.
        pager: String,
        /// Exit status returned by the pager.
        status: ExitStatus,
    },
    /// Failed to write to a pager process.
    #[error("Failed to write to pager `{pager}`: {source}")]
    PagerWrite {
        /// Pager command that failed to receive input.
        pager: String,
        /// Underlying write error.
        source: io::Error,
    },
    /// An interactive prompt was interrupted or canceled.
    #[error("Prompt canceled.")]
    PromptCanceled,
    /// An interactive prompt failed.
    #[error("Prompt failed: {message}")]
    PromptFailed {
        /// Error message describing the prompt failure.
        message: String,
    },
    /// A skill could not be found.
    #[error("Skill not found: {name}")]
    SkillNotFound {
        /// Missing skill name.
        name: String,
    },
    /// A required path already exists.
    #[error("Path already exists: {path}")]
    PathExists {
        /// Path that already exists.
        path: PathBuf,
    },
    /// A path was not a valid file system location.
    #[error("Invalid path: {path}")]
    InvalidPath {
        /// Path that could not be used.
        path: PathBuf,
    },
    /// A path required for operation does not exist.
    #[error("Path does not exist: {path}")]
    PathMissing {
        /// Path that does not exist.
        path: PathBuf,
    },
    /// A skill file could not be written.
    #[error("Failed to write skill file at {path}: {source}")]
    SkillWrite {
        /// Path that failed to write.
        path: PathBuf,
        /// Underlying IO error.
        source: io::Error,
    },
    /// A skill file could not be read.
    #[error("Failed to read skill file at {path}: {source}")]
    SkillRead {
        /// Path that failed to read.
        path: PathBuf,
        /// Underlying IO error.
        source: io::Error,
    },
    /// A template could not be rendered.
    #[error("Failed to render template: {message}")]
    TemplateRender {
        /// Error message describing the render failure.
        message: String,
    },
    /// No local skill was found with the given name.
    #[error("No local skill named '{name}' found in .claude/skills/ or .codex/skills/")]
    LocalSkillNotFound {
        /// Missing skill name.
        name: String,
    },
    /// A skill already exists at the target location.
    #[error("Skill '{name}' already exists at {path}. Use --force to overwrite.")]
    SkillExists {
        /// Skill name.
        name: String,
        /// Path where the skill exists.
        path: PathBuf,
    },
    /// Failed to move a skill directory.
    #[error("Failed to move skill from {from} to {to}: {source}")]
    SkillMove {
        /// Source path.
        from: PathBuf,
        /// Destination path.
        to: PathBuf,
        /// Underlying IO error.
        source: io::Error,
    },
    /// Multiple local skills found with the same name.
    #[error("Multiple local skills named '{name}' found. Specify --tool to disambiguate.")]
    AmbiguousLocalSkill {
        /// Ambiguous skill name.
        name: String,
    },
    /// Failed to create a ZIP archive.
    #[error("Failed to create ZIP archive at {path}: {message}")]
    ZipCreate {
        /// Path to the ZIP file.
        path: PathBuf,
        /// Error message.
        message: String,
    },
    /// Failed to read a ZIP archive.
    #[error("Failed to read ZIP archive at {path}: {message}")]
    ZipRead {
        /// Path to the ZIP file.
        path: PathBuf,
        /// Error message.
        message: String,
    },
    /// Invalid ZIP structure.
    #[error("Invalid ZIP file: {message}")]
    InvalidZip {
        /// Error message describing the invalid structure.
        message: String,
    },
    /// Failed to download from URL.
    #[error("Failed to download from {url}: {message}")]
    Download {
        /// URL that failed.
        url: String,
        /// Error message.
        message: String,
    },
    /// Invalid URL.
    #[error("Invalid URL: {url}")]
    InvalidUrl {
        /// The invalid URL.
        url: String,
    },
    /// HTTP URL not allowed.
    #[error("HTTP URLs not allowed for security. Use HTTPS: {url}")]
    HttpNotAllowed {
        /// The HTTP URL.
        url: String,
    },
    /// File too large.
    #[error("File too large: {size} bytes (max {max} bytes)")]
    FileTooLarge {
        /// Actual size.
        size: u64,
        /// Maximum allowed size.
        max: u64,
    },
    /// Sync conflict between tools.
    #[error("Conflict: skill '{name}' has divergent modifications in {tools}. Resolve manually.")]
    SyncConflict {
        /// Skill name with conflict.
        name: String,
        /// Tools with conflicting changes.
        tools: String,
    },
    /// Failed to launch the editor.
    #[error("Failed to run editor `{editor}`: {message}")]
    EditorFailed {
        /// Editor command that failed.
        editor: String,
        /// Error message.
        message: String,
    },
    /// Validation failed.
    #[error("Validation failed: {message}")]
    ValidationFailed {
        /// Error message describing the validation failure.
        message: String,
    },
}

impl Error {
    /// Map errors to exit codes for CLI termination.
    pub fn exit_code(&self) -> ExitCode {
        ExitCode::from(1)
    }
}
