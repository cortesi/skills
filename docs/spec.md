# Skills CLI Specification

A command-line tool for managing agent skills across Claude Code and OpenAI Codex.

## Overview

The `skills` CLI provides a unified interface for managing skills across multiple AI coding
assistants. Core goals:

- **Unified management** - A single tool to manage skills for all supported platforms
- **Avoid duplication** - Maintain skills in one place, sync to tools as needed
- **Consistency** - Ensure skills stay in sync across tools
- **Extensibility** - Support templating for tool-specific customization

## Supported Platforms

| Platform    | Skills Directory      | Format                         |
|-------------|-----------------------|--------------------------------|
| Claude Code | `~/.claude/skills/`   | SKILL.md with YAML frontmatter |
| Codex       | `~/.codex/skills/`    | SKILL.md with YAML frontmatter |
| Gemini      | `~/.gemini/skills/`   | SKILL.md with YAML frontmatter |

Future versions may add support for additional AI coding tools.

## Configuration

The CLI reads configuration from `~/.skills.toml` on Unix-like systems. No default source
directories are assumed—users must explicitly configure at least one source.

If the config file is missing or `sources` is empty, the CLI exits with code 1 and prints a
clear error such as: "No sources configured; edit ~/.skills.toml to add at least one source."

Source paths are resolved by expanding `~` and environment variables, then resolving relative
paths relative to the config file directory. Path separators are normalized. The CLI does not
require symlink resolution for display, but should prefer canonical paths when comparing for
conflicts if available.

```toml
# ~/.skills.toml

# Source directories in priority order (first wins on conflicts)
sources = [
    "~/dotfiles/skills",
    "~/work/team-skills",
]
```

### Source Priority

When the same skill (by name) exists in multiple source directories, the first directory in the
`sources` list takes priority. A warning is emitted to alert users to the conflict:

```
Warning: skill 'pdf' exists in multiple sources, using ~/dotfiles/skills
  - ~/dotfiles/skills/pdf
  - ~/work/team-skills/pdf
```

## Commands

### `skills list`

Lists all skills with their sync status across tools. Output uses color and may span multiple
lines per skill for clarity.

Output is sorted alphabetically by skill name (case-insensitive), stable within name ties.

Color defaults to `--color=auto` (disabled on non-TTY output). Provide `--color=always` and
`--color=never` to force behavior.

```
$ skills list
pdf
  source: ~/dotfiles/skills
  claude: synced    codex: synced

xlsx
  source: ~/dotfiles/skills
  claude: synced    codex: modified ←

doc-interview
  source: ~/dotfiles/skills
  claude: synced    codex: missing

legacy-tool
  source: -
  claude: orphan    codex: orphan
```

Status indicators:
- `synced` - Tool copy matches source
- `modified` - Tool copy differs from source
- `missing` - Not installed in tool
- `orphan` - Exists in tool but not in any source

### `skills push [skill-name]`

Pushes skills from source directories to tool directories. By default pushes all skills; pass a
skill name to push just one.

When a tool copy is `modified`, the default behavior is to prompt per skill (default No). The
`--force` flag overwrites without prompting.

`synced/modified` status is determined by comparing the rendered template for the target tool to
the installed tool copy, byte-for-byte after normalizing line endings.

```
$ skills push
Pushing Claude Code...
  + pdf (new)
  ~ xlsx (pushed)
Pushing Codex...
  + pdf (new)
  = xlsx (unchanged)

Completed with 1 warning. Use --verbose for details.
```

```
$ skills push pdf
Pushing pdf...
  claude: + (new)
  codex:  + (new)
```

Options:
- `--dry-run` / `-n` - Show what would change without making changes
- `--force` / `-f` - Overwrite modified skills in tool directories without prompting

Orphaned skills (those in tool directories but not in sources) are left untouched.

### `skills pull [skill-name] [--to <source>]`

Pulls modified skills from tool directories back to source directories. Useful when skills are
edited in-place using tool-specific skill editors.

If a skill exists only in a tool directory (orphan), prompt to create it in a chosen source
(default Skip).

If multiple sources are configured and `--to` is not provided, prompt for a target source and
default to the highest-priority source when the user presses Enter.

```
$ skills pull
Found 2 modified skills:

xlsx (modified in Codex)
  source: ~/dotfiles/skills/xlsx/SKILL.md
  tool:   ~/.codex/skills/xlsx/SKILL.md

Pull changes to source? [y/N] y
Pulled xlsx from Codex → ~/dotfiles/skills
```

When a skill is modified differently in multiple tools, the user is prompted to resolve the
conflict interactively:

```
$ skills pull xlsx
xlsx has different modifications in multiple tools:

  [1] Claude Code  (modified 2 hours ago)
  [2] Codex        (modified 30 minutes ago)
  [d] Show diff between versions
  [s] Skip

Which version to pull? [1/2/d/s]
```

Options:
- `--to <source>` - Pull to a specific source directory when multiple are configured

### `skills new <path>`

Creates a new skill from a template at the specified path. The skill name defaults to the folder
name; description is left as a placeholder for the user to fill in.

If the destination path already exists, the command errors and refuses to overwrite by default.

```
$ skills new ~/dotfiles/skills/my-helper
Created skill at ~/dotfiles/skills/my-helper/SKILL.md

Edit the SKILL.md file, then run `skills push` to sync.
```

Generated template:

```markdown
---
name: my-helper
description: <describe when this skill should be used>
---

# My Helper

<instructions for the AI assistant>
```

### `skills init`

Prompts for a skills source directory and writes a config file at `~/.skills.toml`.

If the config file is missing, other commands should automatically invoke `skills init` before
proceeding.

### `skills diff [skill-name]`

Shows detailed differences between source and installed skills.

If `--pager` is not provided, the command falls back to `GIT_PAGER`, `pager.diff`, `core.pager`,
and `PAGER` in that order.

```
$ skills diff xlsx
━━━ xlsx ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Codex: modified
--- source: ~/dotfiles/skills/xlsx/SKILL.md
+++ tool:   ~/.codex/skills/xlsx/SKILL.md
@@ -10,3 +10,5 @@
 ## Guidelines
+
+Always validate cell references before writing formulas.
```

## Templating

Skills can include conditional sections for tool-specific content using
[MiniJinja](https://github.com/mitsuhiko/minijinja) syntax. MiniJinja is a minimal Jinja2-compatible
templating engine that works well with Markdown content. The Jinja2 tags remain visible when
viewing the source Markdown, making it clear which sections are conditional.

Only `SKILL.md` is templated in the initial version.

On template rendering errors, the skill is skipped with a warning and processing continues.

```markdown
## Background Tasks

{% if tool == "claude" %}
Use the Task tool to spawn background agents for long-running operations.
{% endif %}

{% if tool == "codex" %}
Use the /background command to run tasks asynchronously.
{% endif %}

{% if tool == "gemini" %}
Use the @background agent for async tasks.
{% endif %}
```

The `tool` variable is automatically set during sync to `"claude"`, `"codex"`, or `"gemini"`. Additional
user-defined variables are not supported in the initial version but could be added later if needed.

## Error Handling

When a skill has invalid YAML frontmatter or missing required fields:

1. Log a warning with the file path and error
2. Skip the invalid skill
3. Continue processing remaining skills
4. Print a summary of skipped skills at the end

```
$ skills push
Warning: ~/skills/broken/SKILL.md - missing required field 'description'
Pushing Claude Code...
  + pdf (new)

Completed with 1 warning. Use --verbose for details.
```

Warnings do not change the exit code; exit 0 when only warnings occur. Non-zero exit codes are
reserved for fatal errors.

## Installation

```
cargo install skills
```

## Technical Implementation

- **Async runtime** - tokio
- **CLI parsing** - clap
- **Config/frontmatter parsing** - serde with toml
- **Templating** - minijinja
- **Progress bars** - indicatif (when needed)
- **Interactive prompts** - inquire
- **Crate structure** - single crate
- **Repo tasks** - xtask

### Repo Tasks

The repository uses an `xtask` for developer workflows. Add a `cargo xtask tidy` command that
runs the formatter and linter.

## Future Considerations

- **Skill sharing** - Registry or repository for discovering and sharing skills
- **Skill versioning** - Track skill versions, support rollback
- **Validation** - Lint skills for common issues before sync
