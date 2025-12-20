# Plan: Skills Pack and Import Commands

## Background

The Agent Skills specification defines skills as directory structures containing SKILL.md files.
Claude.ai/Desktop expects skills as standard ZIP files with `.zip` extension for upload.

This plan proposes two commands:
- `skills pack` - Package skills into ZIP files for sharing
- `skills import` - Import skills from ZIP files or URLs

## Research Findings

### Claude.ai/Desktop Format (from support.claude.com)

Skills are shared as **ZIP files** with the skill folder as the root element:

```
my-skill.zip
└── my-skill/
    ├── Skill.md              # Required (note: capital S)
    └── resources/            # Optional
```

**SKILL.md Frontmatter (required fields):**
```yaml
---
name: skill-name           # Max 64 chars
description: "..."         # Max 200 chars, explains when to use
---
```

**Optional content:**
- Reference files (e.g., REFERENCE.md)
- Executable code (Python, JavaScript/Node.js)
- Resource folders (logos, fonts, templates)

### Claude Code / Codex CLI Format

These tools use `SKILL.md` (all caps) in directory structures:
- Claude Code: `~/.claude/skills/` or `.claude/skills/`
- Codex CLI: `~/.codex/skills/`

**Note:** There's a capitalization difference - Claude.ai uses `Skill.md`, while Claude Code uses
`SKILL.md`. Our tools should accept both when importing and preserve the original when packing.

## ZIP File Format

Standard ZIP archive with `.zip` extension:

```
my-skill.zip
└── my-skill/                    # Root directory matches skill name
    ├── SKILL.md                 # Required (accept Skill.md or SKILL.md)
    ├── scripts/                 # Optional - executable code
    ├── references/              # Optional - supporting documentation
    ├── assets/                  # Optional - templates, data files
    └── resources/               # Optional - configuration
```

**Why standard .zip?**
- Directly compatible with Claude.ai/Desktop upload
- No renaming needed
- Universal tooling support
- Human-inspectable

## Design

### `skills pack` Command

Package one or all skills into ZIP files for sharing.

```bash
# Pack a single source skill
skills pack code-review

# Pack to specific output path
skills pack code-review -o ~/Desktop/code-review.zip

# Pack ALL source skills (creates multiple .zip files)
skills pack --all
skills pack --all -o ~/Desktop/

# Pack a local skill from current project
skills pack --local project-helper

# Pack with validation (check SKILL.md format)
skills pack code-review --validate
```

**Behavior (single skill):**
1. Find the skill by name in sources (or local if `--local`)
2. Validate SKILL.md has required frontmatter fields
3. Create ZIP archive with skill directory at root
4. Write to `<skill-name>.zip` in current directory (or `-o` path)

**Behavior (--all):**
1. Load all skills from configured sources
2. For each skill:
   - Validate SKILL.md
   - Create `<skill-name>.zip`
3. Write all ZIP files to current directory (or `-o` directory)
4. Report summary of packed skills

**Output (single):**
```
Packing 'code-review' from ~/dotfiles/skills/code-review

Created: ./code-review.zip (2.3 KB)
  - SKILL.md
  - scripts/review.py
  - references/guidelines.md

Share this file or import with: skills import code-review.zip
```

**Output (--all):**
```
Packing all skills from configured sources...

  ✓ code-review.zip (2.3 KB)
  ✓ doc-writer.zip (1.1 KB)
  ✓ test-helper.zip (856 B)
  ✗ broken-skill (skipped: missing 'description' field)

Created 3 skill archives in ./
```

**Flags:**
- `-o, --output <path>` - Output path (file for single, directory for --all)
- `--all` - Pack all skills from configured sources
- `--local` - Pack from local project skills instead of sources
- `--validate` - Validate SKILL.md format before packing (default: on)
- `--no-validate` - Skip validation
- `-n, --dry-run` - Show what would be packed without creating files
- `-f, --force` - Overwrite existing ZIP files

### `skills import` Command

Import a skill from a ZIP file, URL, or GitHub repository.

```bash
# Import from local file
skills import code-review.zip

# Import from URL
skills import https://example.com/skills/code-review.zip

# Import from GitHub (directory in a repo)
skills import https://github.com/user/repo/tree/main/my-skill

# Import to specific location only
skills import code-review.zip --to claude
skills import code-review.zip --to codex
skills import code-review.zip --to source

# Import as local project skill
skills import code-review.zip --local

# Preview without importing
skills import code-review.zip --dry-run

# Force overwrite existing skill
skills import code-review.zip --force
```

**Behavior:**
1. Detect source type (local file, URL, or GitHub URL)
2. If GitHub URL, use GitHub API to download directory as ZIP
3. If URL, download to temporary file
4. Validate ZIP structure (must have single root directory with SKILL.md or Skill.md)
5. Parse frontmatter to get skill name
6. Check if skill already exists at target locations
7. Extract to target location(s)

**Default Target:** By default, import extracts to **both** global skill directories:
- `~/.claude/skills/<skill-name>/`
- `~/.codex/skills/<skill-name>/`

This makes the skill immediately available in both Claude Code and Codex. Users can then use
`skills pull` to bring it into a source directory for versioned management if desired.

**Output:**
```
Importing 'code-review' from code-review.zip

Extracting to:
  ~/.claude/skills/code-review/
  ~/.codex/skills/code-review/

Contents:
  - SKILL.md
  - scripts/review.py
  - references/guidelines.md

Done. Skill is now available in Claude Code and Codex.
To manage in your source directory: skills pull code-review
```

**Flags:**
- `--to <target>` - Import to specific location only: `claude`, `codex`, `source`, or a path
- `--local` - Import as local project skill (to `.claude/skills/` and `.codex/skills/`)
- `-f, --force` - Overwrite existing skill without prompting
- `-n, --dry-run` - Show what would be imported without extracting
- `--validate` - Validate SKILL.md format after extraction

### GitHub URL Support

Import supports GitHub URLs pointing to skill directories:

```bash
skills import https://github.com/anthropics/skills/tree/main/skills/pdf
```

**Detection:** URLs matching `github.com/<owner>/<repo>/tree/<ref>/<path>` are treated as GitHub
directory references.

**Implementation:** Use GitHub's API to download the repository ZIP:
```
GET https://api.github.com/repos/<owner>/<repo>/zipball/<ref>
```
Then extract only the relevant subdirectory from the downloaded archive.

### Error Cases

**Pack errors:**
- `Skill not found: <name>` - skill doesn't exist in sources
- `Missing required field 'name' in SKILL.md` - invalid frontmatter
- `Output path already exists: <path>` - use `--force` to overwrite

**Import errors:**
- `Invalid ZIP file: no root directory found` - bad structure
- `Invalid ZIP file: missing SKILL.md` - no SKILL.md in archive
- `Skill '<name>' already exists at <path>. Use --force to overwrite.`
- `Failed to download: <url>` - network error
- `Invalid URL: <url>` - malformed URL
- `GitHub API error: <message>` - rate limit, not found, etc.

### URL Import Security

When importing from URLs:
1. Only HTTPS URLs allowed (HTTP rejected with warning)
2. Download to temporary directory first
3. Validate ZIP structure before extraction
4. Check for path traversal attacks (../ in filenames)
5. Limit maximum file size (default 10MB, configurable)
6. Validate SKILL.md frontmatter before completing import

## Implementation Plan

### Phase 1: Pack Command (Single Skill)
- [ ] Add `pack` subcommand to CLI
- [ ] Implement skill directory → ZIP archive creation
- [ ] Add SKILL.md validation
- [ ] Support `--local`, `--output`, `--dry-run`, `--validate`, `--force` flags

### Phase 2: Pack Command (--all)
- [ ] Implement `--all` flag to pack all source skills
- [ ] Generate multiple ZIP files
- [ ] Report summary with success/failure counts

### Phase 3: Import Command (Local Files)
- [ ] Add `import` subcommand to CLI
- [ ] Implement ZIP archive validation (accept both Skill.md and SKILL.md)
- [ ] Implement extraction to both global skill directories by default
- [ ] Support `--to`, `--local`, `--force`, `--dry-run` flags

### Phase 4: Import Command (URLs)
- [ ] Add URL detection and download
- [ ] Implement HTTPS-only requirement
- [ ] Add file size limits
- [ ] Add path traversal protection

### Phase 5: GitHub URL Support
- [ ] Add GitHub URL pattern detection
- [ ] Implement GitHub API integration for directory download
- [ ] Handle API rate limiting and errors
- [ ] Extract subdirectory from full repo ZIP

### Phase 6: Polish
- [ ] Add progress indicators for large files/downloads
- [ ] Add `--quiet` flag for scripting
- [ ] Consider checksum verification for URLs

## Example Workflows

### Sharing a skill

```bash
# Create and test a skill locally
skills new ~/skills/my-helper
# ... edit SKILL.md and add scripts ...
skills push my-helper

# Package for sharing
skills pack my-helper
# → Creates ./my-helper.zip

# Or pack all your skills at once
skills pack --all -o ~/skill-backups/
# → Creates ~/skill-backups/my-helper.zip, ~/skill-backups/other-skill.zip, etc.

# Share via email, Slack, or upload to Claude.ai
```

### Importing a shared skill

```bash
# Import from file - goes to both ~/.claude/skills/ and ~/.codex/skills/
skills import my-helper.zip

# Import from URL
skills import https://example.com/my-helper.zip

# Import from GitHub
skills import https://github.com/anthropics/skills/tree/main/skills/pdf

# To manage the skill in your source directory afterward:
skills pull my-helper --to ~/my-skills
```

### Importing as local project skill

```bash
# Import for current project only (both .claude/skills/ and .codex/skills/)
skills import team-conventions.zip --local
# → Extracts to ./.claude/skills/team-conventions/
# → Extracts to ./.codex/skills/team-conventions/
# → Available immediately in this project for both tools
```

### Importing to specific tool only

```bash
# Only install for Claude Code
skills import my-helper.zip --to claude

# Only install for Codex
skills import my-helper.zip --to codex

# Install to a source directory for management
skills import my-helper.zip --to ~/my-skills
```

### Backing up all skills

```bash
# Pack all skills for backup
skills pack --all -o ~/Dropbox/skill-backups/

# Later, restore on a new machine
for f in ~/Dropbox/skill-backups/*.zip; do
  skills import "$f"
done
```
