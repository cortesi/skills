//! YAML frontmatter parsing for skill files.

use serde::Deserialize;

/// Parsed frontmatter fields from a skill file.
#[derive(Debug, Clone)]
pub struct Frontmatter {
    /// The declared skill name.
    pub name: String,
    /// The declared skill description.
    pub description: String,
}

/// Raw frontmatter fields for validation.
#[derive(Debug, Deserialize)]
struct RawFrontmatter {
    /// The declared skill name.
    name: Option<String>,
    /// The declared skill description.
    description: Option<String>,
}

/// Errors that can occur when parsing frontmatter.
#[derive(Debug, Clone)]
pub struct FrontmatterError {
    /// A human-readable error message.
    pub message: String,
}

impl FrontmatterError {
    /// Create a new frontmatter error message.
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Parse the frontmatter from a skill file.
pub fn parse_frontmatter(contents: &str) -> Result<Frontmatter, FrontmatterError> {
    let bounds = frontmatter_bounds(contents)
        .ok_or_else(|| FrontmatterError::new("missing YAML frontmatter"))?;
    let frontmatter = &contents[bounds.start..bounds.end];
    let raw: RawFrontmatter = serde_yaml::from_str(frontmatter)
        .map_err(|error| FrontmatterError::new(error.to_string()))?;

    let name = raw.name.unwrap_or_default().trim().to_string();
    if name.is_empty() {
        return Err(FrontmatterError::new("missing required field 'name'"));
    }

    let description = raw.description.unwrap_or_default().trim().to_string();
    if description.is_empty() {
        return Err(FrontmatterError::new(
            "missing required field 'description'",
        ));
    }

    Ok(Frontmatter { name, description })
}

/// Byte range bounds for frontmatter in a document.
#[derive(Debug, Clone, Copy)]
struct FrontmatterBounds {
    /// Start byte index of the YAML payload.
    start: usize,
    /// End byte index of the YAML payload.
    end: usize,
}

/// Locate the byte range containing frontmatter in a document.
fn frontmatter_bounds(contents: &str) -> Option<FrontmatterBounds> {
    let mut offset = 0;
    let mut lines = contents.split_inclusive('\n');
    let first = lines.next()?;
    if trim_line_endings(first) != "---" {
        return None;
    }
    offset += first.len();
    let start = offset;

    for line in lines {
        if trim_line_endings(line) == "---" {
            return Some(FrontmatterBounds { start, end: offset });
        }
        offset += line.len();
    }

    None
}

/// Trim CRLF and LF suffixes from a line fragment.
fn trim_line_endings(line: &str) -> &str {
    line.trim_end_matches(['\r', '\n'])
}

#[cfg(test)]
mod tests {
    use super::{FrontmatterError, parse_frontmatter};

    fn parse_error(contents: &str) -> FrontmatterError {
        parse_frontmatter(contents).expect_err("frontmatter should fail")
    }

    #[test]
    fn rejects_missing_frontmatter() {
        let error = parse_error("# Title\n");
        assert_eq!(error.message, "missing YAML frontmatter");
    }

    #[test]
    fn rejects_missing_fields() {
        let contents = "---\nname: example\n---\n";
        let error = parse_error(contents);
        assert_eq!(error.message, "missing required field 'description'");
    }

    #[test]
    fn parses_required_fields() {
        let contents = "---\nname: example\ndescription: test\n---\nBody";
        let parsed = parse_frontmatter(contents).expect("frontmatter should parse");
        assert_eq!(parsed.name, "example");
    }
}
