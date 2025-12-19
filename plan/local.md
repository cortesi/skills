# Plan: Comprehensive Local Skills Support

**Status: Implemented**

## Background

Claude Code and Codex both support **local skills** - skills stored within a project directory
(`.claude/skills/` or `.codex/skills/`) that are checked into version control and available to all
developers working on that project.

This plan addresses how the `skills` CLI should handle these local skills comprehensively.

## Current State

The `skills` CLI currently manages skills from **centralized sources** configured in
`~/.skills.toml`. It syncs these source skills to:
- `~/.claude/skills/` (Claude personal skills)
- `~/.codex/skills/` (Codex personal skills)

**Local skills** (`.claude/skills/` and `.codex/skills/` in project directories) are not currently
handled.

## Key Concepts

### Skill Locations

| Location | Scope | Managed by `skills` CLI? |
|----------|-------|--------------------------|
| `~/.skills.toml` sources | Personal (all projects) | Yes - source of truth |
| `~/.claude/skills/` | Personal (all projects) | Yes - push target |
| `~/.codex/skills/` | Personal (all projects) | Yes - push target |
| `.claude/skills/` | Project (git-tracked) | Yes - detection + uplift |
| `.codex/skills/` | Project (git-tracked) | Yes - detection + uplift |

### Terminology

- **Source skills**: Skills in configured source directories (`~/.skills.toml`)
- **Personal skills**: Skills installed to `~/.claude/skills/` or `~/.codex/skills/`
- **Local skills**: Skills in `.claude/skills/` or `.codex/skills/` within a project directory
- **Skills store**: A source directory configured in `~/.skills.toml`
- **Uplift**: Move a local skill to the matching global location (making it available globally)

## Design

### 1. Local Skills Detection

When running `skills` commands within a project directory, detect local skills by looking for
`.claude/skills/` and `.codex/skills/` directories relative to the current working directory.

```rust
struct LocalSkill {
    name: String,
    tool: Tool,                    // Claude or Codex
    skill_path: PathBuf,           // Full path to SKILL.md
    skill_dir: PathBuf,            // Directory containing SKILL.md
}

fn detect_local_skills(cwd: &Path) -> Vec<LocalSkill> {
    // Check for .claude/skills/ and .codex/skills/ in cwd
    // Load any SKILL.md files found in subdirectories
    // Parse frontmatter to extract name
}
```

### 2. Enhanced `skills list` Command

Show local skills in a separate section from personal skills:

```
Source Skills:
  code-review     [claude: synced] [codex: synced]
  doc-writer      [claude: synced] [codex: missing]

Local Skills:
  project-setup   [claude]
  deploy-helper   [claude]
  codex-helper    [codex]

Conflicts:
  ⚠ 'code-review' exists locally and globally
    Local takes precedence in this project
```

### 3. Conflict Detection and Warnings

When a local skill has the same name as a personal skill, warn the user and indicate precedence:

```
Warning: Local skill 'code-review' shadows personal skill
  Local:    ./.claude/skills/code-review/SKILL.md
  Personal: ~/.claude/skills/code-review/SKILL.md

  Claude will use the local version in this project.
```

Local skills take precedence over personal skills when both exist with the same name (this is how
Claude Code and Codex behave).

### 4. New `skills uplift` Command

Move a local skill to the matching global location:

```bash
# Move local skill to global (interactive if multiple local skills match)
skills uplift project-setup

# Dry run - show what would happen
skills uplift project-setup --dry-run

# Force overwrite if skill already exists globally
skills uplift project-setup --force
```

**Behavior:**
1. Find the local skill by name in `.claude/skills/` or `.codex/skills/`
2. Determine the target: `.claude/skills/X` → `~/.claude/skills/X`, `.codex/skills/X` →
   `~/.codex/skills/X`
3. Check if skill already exists at target location
   - If exists and no `--force`: error with message suggesting `--force`
   - If exists and `--force`: overwrite
4. Move the skill directory to the global location (not copy)
5. Print success message with hint about using `skills pull` to manage as a source skill

**Example output:**
```
Uplifting 'project-setup' from .claude/skills/ to ~/.claude/skills/

Moving: ./.claude/skills/project-setup/ → ~/.claude/skills/project-setup/

Done. To manage this skill from your source directory, run:
  skills pull project-setup --to ~/your-skills-source
```

**Error cases:**
- Skill not found locally: `Error: No local skill named 'project-setup' found`
- Skill exists globally: `Error: Skill 'project-setup' already exists at ~/.claude/skills/. Use
  --force to overwrite.`
- Multiple local skills with same name (in both .claude and .codex): prompt user to specify which
  one

## Implementation Plan

### Phase 1: Local Skills Detection
- [x] Add `LocalSkill` struct to represent local skills
- [x] Implement `detect_local_skills()` function to scan `.claude/skills/` and `.codex/skills/`
- [x] Add local skills to `Catalog` struct (separate from sources/tools)

### Phase 2: Enhanced List Command
- [x] Modify `skills list` to show local skills in a separate section
- [x] Implement conflict detection between local and personal skills
- [x] Show warnings for conflicts with precedence information

### Phase 3: Uplift Command
- [x] Add `uplift` subcommand to CLI
- [x] Implement skill lookup in local directories
- [x] Implement move operation with proper error handling
- [x] Support `--dry-run` flag for preview
- [x] Support `--force` flag to overwrite existing global skills
- [x] Handle ambiguous cases (skill exists in both .claude and .codex locally)

## Design Decisions

### Why move instead of copy?

Uplift moves the skill rather than copying because:
1. Avoids duplicate maintenance burden
2. Clear ownership - skill is now managed globally
3. User can use `skills pull` to bring it into a source directory for versioned management
4. Prevents confusion about which copy is authoritative

### Why NOT sync local skills automatically?

Local skills are checked into git and managed as part of the project. They should NOT be
automatically synced because:

1. They may be project-specific and irrelevant globally
2. Different projects may have conflicting skills with the same name
3. The project's git repo is already the source of truth
4. Automatic syncing would be surprising and potentially destructive

### Why no `localize` command (inverse of uplift)?

Users can manually copy skills to `.claude/skills/` if needed. The common workflow is:
1. Create local skill in project (manual or `skills new --local`)
2. Iterate and refine within the project
3. Uplift to global when ready to share across projects
4. Pull to source directory for long-term management

The reverse flow (global → local) is uncommon and doesn't warrant a dedicated command.

### Handling both Claude and Codex local skills

For consistency, we handle both `.claude/skills/` and `.codex/skills/` identically:
- Both are detected and shown in `skills list`
- Both can be uplifted to their respective global locations
- Conflicts are checked independently for each tool

## Appendix: Tool Behavior Reference

### Claude Code
- Personal: `~/.claude/skills/skill-name/SKILL.md`
- Project: `.claude/skills/skill-name/SKILL.md`
- Project skills take precedence over personal skills

### Codex CLI
- Personal: `~/.codex/skills/skill-name/SKILL.md`
- Project: `.codex/skills/skill-name/SKILL.md`
- Uses same SKILL.md format as Claude
- Both tools use progressive disclosure (metadata first, full content on-demand)
