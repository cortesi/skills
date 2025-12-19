# Skills

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

| Command | What it does |
|---------|--------------|
| `skills list` | Show all skills and their sync status |
| `skills push` | Sync skills from source to tools |
| `skills pull` | Pull edits from tools back to source |
| `skills diff` | Show differences between source and installed |
| `skills new <path>` | Create a new skill skeleton |
| `skills init` | Set up your configuration |

### Typical Workflow

Create or edit a skill in Claude Code (it has a built-in skill editor), then sync:

```bash
skills pull          # Pull the new skill to your repo
skills push          # Push it to Codex
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

## Supported Tools

| Tool | Skills Directory | Documentation |
|------|------------------|---------------|
| Claude Code | `~/.claude/skills/` | [docs](https://code.claude.com/docs/en/skills) |
| Codex | `~/.codex/skills/` | [docs](https://github.com/openai/codex/blob/main/docs/skills.md) |

## License

MIT
