# Skills CLI Phase One Plan

This plan delivers the end-to-end first-phase CLI described in docs/spec.md, including config
parsing, skill discovery, templating, and all listed commands for Claude and Codex.

Phase one includes the full command set in the spec. Add Windows support for config and path
handling. `skills diff` should support a user-specified pager (for example `--pager delta`) and a
colorized unified diff when no pager is specified.

0. Stage Zero: Dev workflow bootstrap

Add developer workflow tooling early so it can be used throughout implementation.

1. [x] Add `xtask` crate and `cargo xtask tidy` to run formatter + linter as specified.

1. Stage One: Foundations and config

Establish the crate structure, dependencies, and core parsing/rendering helpers.

1. [x] Add core dependencies via `cargo add` and create module layout with a shared error type.
2. [x] Implement config loading for `~/.skills.toml` with cross-platform home resolution, env
   expansion, and normalized paths (Unix + Windows).
3. [x] Implement skill model, YAML frontmatter parsing, and template rendering with `tool` var.

2. Stage Two: Discovery and status engine

Build the internal index of skills across sources and tools, with conflict handling.

1. [x] Scan source directories for skills, detect name conflicts, and emit priority warnings.
2. [x] Scan Claude/Codex tool directories, detect orphans, and map tool installs per skill.
3. [x] Compute sync status with line-ending normalization and stable case-insensitive sorting.

3. Stage Three: `skills list`

Expose the status engine via list output with color handling.

1. [x] Implement `--color` handling (`auto/always/never`) with TTY detection for output.
2. [x] Implement `skills list` formatting and ensure the multi-line layout matches the spec.
3. [x] Add tests for list ordering and color-disabled output.

4. Stage Four: `skills update`

Sync source skills to tools with dry-run and overwrite behavior.

1. [x] Implement update planning logic with `--dry-run`, per-tool summaries, and `--force`.
2. [x] Add interactive prompts for modified skills (default No) using `inquire`.
3. [x] Write tool copies to disk, skip orphans, and emit warnings + completion summaries.

5. Stage Five: `skills pull`

Pull modified skills from tools back into sources with conflict resolution.

1. [x] Detect modified skills per tool, including orphans, and present pull candidates.
2. [x] Implement interactive selection for tool conflicts and target sources, plus `--to`.
3. [x] Apply pulls to sources, handle skips, and print final summaries.

6. Stage Six: `skills diff`

Show unified diffs between source and tool copies.

1. [x] Choose a diff library/approach and implement colorized unified diff formatting.
2. [x] Add `--pager` handling for `skills diff` and wire pager output when specified.
3. [x] Implement `skills diff [skill]` output with per-tool headers and statuses.

7. Stage Seven: `skills new`

Generate new skill scaffolding with safe defaults.

1. [x] Implement `skills new <path>` scaffolding with name inference and template content.
2. [x] Add error handling for existing destinations and invalid paths.

8. Stage Eight: Validation and docs

Finish with developer tooling, docs, and validation.

1. [x] Add tests for config errors, frontmatter validation, template rendering, and status.
2. [x] Update README/docs with phase-one usage examples and known limitations.
3. [x] Run formatter, clippy, and tests to validate the full workflow.

9. Stage Nine: Config bootstrap

Add an init command and auto-setup when config is missing.

1. [x] Implement `skills init` to prompt for a source directory and create `~/.skills.toml`.
2. [x] Auto-run init when other commands start without a config file.
