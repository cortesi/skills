# Skills CLI

A command-line tool for managing agent skills across Claude Code and OpenAI Codex.

## Configuration

The CLI reads configuration from a single file:

- Unix-like systems: `~/.skills.toml`
- Windows: `%USERPROFILE%\.skills.toml`

The config file must define at least one source directory.

```toml
# ~/.skills.toml
sources = [
    "~/dotfiles/skills",
    "~/work/team-skills",
]
```

Source paths expand `~` and `$VARS`. Relative paths resolve from the config file directory.

## Commands

List skills and status:

```bash
skills list
```

Initialize a config file (prompting for a source directory):

```bash
skills init
```

Update tool installs from sources (all skills, dry-run, or a single skill):

```bash
skills update
skills update --dry-run
skills update pdf
```

Pull tool edits back into sources:

```bash
skills pull
skills pull xlsx
skills pull --to ~/dotfiles/skills
```

Show diffs (optionally via a pager):

```bash
skills diff
skills diff xlsx --pager delta
```

Create a new skill skeleton:

```bash
skills new ~/dotfiles/skills/my-helper
```

## Templating

`SKILL.md` files in sources may include MiniJinja templating. The `tool` variable is set to
`claude` or `codex` when rendering.

## Limitations

- Only Claude Code and Codex are supported in phase one.
- `skills pull` copies the tool version into sources; template structure is not preserved.
- Pager commands are parsed with shell-style splitting; quote arguments as needed.
- Most commands will prompt to run `skills init` if no config is present.
