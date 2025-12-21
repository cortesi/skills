# Skills CLI Ergonomic Improvements

Improve the skills CLI interface to be more consistent, intuitive, and powerful. Changes focus on
command naming consistency, symmetry in operations, reducing cognitive load, and adding missing
functionality.

## 1. Stage One: Command Symmetry & Defaults

Make push behave consistently with pull/diff by supporting multiple skills and operating on all
skills when no argument is given.

1. [x] Update `push` command to accept multiple skill names (like `pack` does)
2. [x] Add `--all` flag to `push` command for explicit "push everything"
3. [x] Make `push` with no arguments default to pushing all out-of-sync skills (with confirmation)
4. [x] Update `skills` with no subcommand to run `list` instead of showing help
5. [x] Add `status` as an alias for `list` (familiar to git users)

## 2. Stage Two: Command Naming Consistency

Rename commands to use clearer, more intuitive terminology.

1. [x] Rename `uplift` to `promote` (clearer direction: local → global)
2. [x] Merge `pack-all` into `pack --all` flag, deprecate `pack-all` command
3. [x] Update `pack` to default to all skills when no skill names provided

## 3. Stage Three: Consistent Flag Patterns

Standardize how tool selection and common options work across all commands.

1. [x] Replace `--claude`/`--codex` flags with unified `--tool <TOOL>` across all commands
       (accepts: `claude`, `codex`, `all`, `both`)
2. [x] Rename `--local` to `--project` for clarity (means "project directory" not "local machine")
3. [x] Simplify `import --to` by splitting into `--to-tool`, `--to-source`, or accepting path
       directly
4. [x] Ensure `-n`/`--dry-run` is available on ALL mutating commands
5. [x] Add `-f`/`--force` consistently to all commands that can conflict

## 4. Stage Four: New Utility Commands

Add missing commands that improve common workflows.

1. [x] Add `skills edit <SKILL>` - opens skill in `$EDITOR` (searches source, then tools)
2. [x] Add `skills validate [SKILL]` - check SKILL.md structure, frontmatter, template syntax
3. [x] Add `skills render <SKILL> --tool <TOOL>` - preview rendered output for specific tool
4. [x] Add `skills mv <OLD> <NEW>` - rename skill across source and all tools

## 5. Stage Five: Workflow Improvements

Reduce friction in common multi-step workflows.

1. [x] Allow `sync` command to accept optional skill name(s) for targeted sync
2. [x] Add `--prefer-source` and `--prefer-tool` flags to `sync` for conflict resolution

## 6. Stage Six: Safety Improvements

Prevent accidental data loss and improve destructive operation UX.

1. [x] Show diff before `push --force` overwrites modified skill
2. [x] Require `--yes` in addition to `--force` for truly silent overwrites
3. [x] Add confirmation prompt to `unload` by default (currently requires `-f` to skip)

## 7. Stage Seven: Documentation & Help

Update documentation to reflect all changes.

1. [x] Update README.md with new command names and flags
2. [x] Update `--help` text for all modified commands
3. [x] Add examples section to help output for complex commands
