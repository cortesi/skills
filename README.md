![Discord](https://img.shields.io/discord/1381424110831145070?style=flat-square&logo=rust&link=https%3A%2F%2Fdiscord.gg%2FfHmRmuBDxF)
[![Crates.io](https://img.shields.io/crates/v/skills)](https://crates.io/crates/skills)

A CLI for managing AI coding assistant skills from a single source.

Both [Claude Code](https://docs.anthropic.com/en/docs/claude-code/skills) and
[Codex](https://github.com/openai/codex/blob/main/docs/skills.md) support skills—reusable
instructions that extend what the assistant can do. This tool keeps them in sync across tools,
so you can maintain skills in one place (your dotfiles, a team repo) and push them everywhere.

- **One source** — No more copying files between `~/.claude/skills/` and `~/.codex/skills/`
- **Sync everywhere** — Push to both tools with a single command
- **Pull edits back** — Edit a skill in Claude Code, pull it to your repo, push to Codex
- **Team sharing** — Point everyone at a shared repo; they all get the same skills
- **Version control** — Skills live in git, so you get history and can roll back

## Quick Start

```bash
cargo install skills

skills init          # Set up ~/.skills.toml
skills list          # See what you have
skills push          # Push skills to your tools
```

## How It Works

Point the CLI at your source directories in `~/.skills.toml`:

```toml
sources = [
    "~/dotfiles/skills",
    "~/work/team-skills",
]
```

Each skill is a folder with a `SKILL.md` file:

```
code-review/
  SKILL.md
```

```markdown
---
name: code-review
description: Use when reviewing pull requests or code changes
---

# Code Review

When reviewing code, focus on:
- Correctness and edge cases
- Readability and maintainability
- Performance implications
- Security considerations

Provide specific, actionable feedback with line references.
```

## Commands

### Core Commands

| Command | What it does |
|---------|--------------|
| `skills list` | Show all skills and their sync status (aliases: `ls`, `status`) |
| `skills push [SKILLS...]` | Push skills from source to tools |
| `skills pull [SKILL]` | Pull edits from tools back to source |
| `skills sync [SKILLS...]` | Two-way sync based on timestamps |
| `skills diff [SKILL]` | Show differences between source and installed |

### Skill Management

| Command | What it does |
|---------|--------------|
| `skills new <path>` | Create a new skill skeleton |
| `skills edit <skill>` | Open a skill in your editor ($EDITOR) |
| `skills mv <old> <new>` | Rename a skill across source and tools |
| `skills validate [SKILL]` | Check skill structure and template syntax |
| `skills render <skill> --tool <tool>` | Preview rendered output for a tool |

### Sharing & Import

| Command | What it does |
|---------|--------------|
| `skills pack [SKILLS...]` | Package skills into ZIP files for sharing |
| `skills import <source>` | Import from ZIP file, URL, or GitHub |
| `skills unload <skill>` | Remove a skill from tool directories |
| `skills promote <skill>` | Move a local skill to global directory (alias: `uplift`) |

### Setup

| Command | What it does |
|---------|--------------|
| `skills init` | Set up your configuration |

### Common Flags

- `--tool <tool>` — Target specific tool: `claude`, `codex`, or `all` (default)
- `--project` — Work with project-local skills (`.claude/skills/`, `.codex/skills/`)
- `-n, --dry-run` — Preview changes without writing
- `-f, --force` — Skip confirmation prompts (shows diff for overwrites)
- `-y, --yes` — Skip all prompts (use with `--force` for fully silent operation)

### Typical Workflow

Create or edit a skill in Claude Code (it has a built-in skill editor), then sync:

```bash
skills pull          # Pull the new skill to your repo
skills push          # Push it to Codex
```

Or use two-way sync:

```bash
skills sync          # Sync based on timestamps
```

Handle conflicts:

```bash
skills sync --prefer-source   # Source wins on conflict
skills sync --prefer-tool     # Newest tool version wins
```

## Templating

Skills can include tool-specific sections using [MiniJinja](https://github.com/mitsuhiko/minijinja):

```markdown
{% if tool == "claude" %}
Use the Task tool for background operations.
{% elif tool == "codex" %}
Use /background for async tasks.
{% endif %}
```

Validate templates before pushing:

```bash
skills validate my-skill
skills render my-skill --tool claude
```

## Supported Tools

| Tool | Skills Directory | Documentation |
|------|------------------|---------------|
| Claude Code | `~/.claude/skills/` | [docs](https://code.claude.com/docs/en/skills) |
| Codex | `~/.codex/skills/` | [docs](https://github.com/openai/codex/blob/main/docs/skills.md) |


## Community

Want to contribute? Have ideas or feature requests? Come tell me about it on
[Discord](https://discord.gg/fHmRmuBDxF). 


## License

MIT
