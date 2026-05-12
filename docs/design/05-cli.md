# CLI -- sensei

## Overview

`sensei` is a Rust binary (`crates/cli/`) for manual operations and benchmarking. It is optional -- the daemon and MCP server handle all automated workflows. The CLI is for developers who want direct control over initialization, status checks, library caching, and hook management. See [ideas/02-setup](../ideas/02-setup.md) for the setup flow this CLI implements.

## Commands

### `sensei init`

Initialize a project for sensei.

1. Detect if `.sensei/` already exists -- warn and suggest `sensei add` if so.
2. Trigger indexing of the current repository.
3. Write `.sensei/config.yaml` with project defaults.
4. Prompt: install pre-commit drift hook? (y/n).
5. If yes: install the hook.
6. Print summary: files created, hook status.

Idempotent -- running twice is safe. Completes with no more than 3 user prompts.

### `sensei add`

Add sensei to a repo that was partially set up.

1. Trigger indexing (safe, does not overwrite existing data).
2. Create `.sensei/config.yaml` with defaults if it does not exist.
3. Report what was added and what was skipped (already existed).

### `sensei upgrade`

Upgrade sensei and refresh project data.

1. Pull latest version.
2. Rebuild.
3. Re-index the current repo (force refresh).
4. Regenerate derived files (non-destructive merge for project config).
5. Print changelog of what changed.

### `sensei status`

Show the health of the current project's sensei setup.

```
sensei status -- /path/to/current/repo
--------------------------------------
Config:     ~/.sensei/config.yaml       ok
Project:    .sensei/config.yaml         ok
Daemon:     senseid                     running (pid 12345)
MCP:        sensei-mcp                  registered
Index:      last updated 2 hours ago
Drift:      3 files drifted (run: sensei drift)
Budget:     $1.23 / $5.00 daily
```

### `sensei guidelines`

View and edit project guidelines.

- `sensei guidelines` -- print merged guidelines (personal + project), each section labeled by source.
- `sensei guidelines edit` -- open guidelines file in `$EDITOR`, wait for close, print confirmation.
- `sensei guidelines show <section>` -- print a specific section by H2 heading.

### `sensei cache`

Manage the shared library cache for library intelligence.

- `sensei cache add <path> [--as <name>]` -- register a library path, trigger indexing, add entry to `.sensei/config.yaml` under `custom_libs`.
- `sensei cache list` -- list cached libraries with file counts and last indexed time.
- `sensei cache update` -- re-index all cached libraries.

### `sensei hooks`

Manage git hook integrations.

- `sensei hooks install [--drift]` -- write a pre-commit hook for drift detection to `.git/hooks/pre-commit`.
- The hook script gracefully skips if `sensei` is not installed, so it does not break for teammates without sensei.

## Profile system

Two-layer configuration with merge semantics.

### Global profile

`~/.sensei/config.yaml` -- developer-level settings shared across all projects.

```yaml
editor: $EDITOR
default_budget:
  daily_limit: 5.00
  monthly_limit: 50.00
```

### Project profile

`<repo>/.sensei/config.yaml` -- project-specific overrides.

```yaml
custom_libs:
  - name: rokkit
    path: ~/Developer/rokkit
ranking_strategy: default
```

### Merge strategy

Project config overrides global config at the key level. Nested objects are merged recursively. Lists are replaced (not appended). If a key exists in both global and project config, the project value wins.

## Hook integration

### Pre-commit drift detection

`sensei hooks install --drift` writes to `.git/hooks/pre-commit`:

```bash
#!/bin/sh
# Drift detection pre-commit hook
# Installed by: sensei hooks install --drift

if ! command -v sensei &> /dev/null; then
  echo "sensei not installed. Skipping drift check."
  exit 0
fi

sensei drift --fail-on-drift
exit $?
```

The hook exits 0 (allowing the commit) if sensei is not installed. When sensei is present, `drift --fail-on-drift` exits non-zero if drifted files are detected, blocking the commit until the drift is resolved.

## Interactive UX

All interactive prompts use terminal prompt patterns:

- **Intro / outro** -- styled session headers for command start and end.
- **Text input** -- free-text for project names, paths.
- **Confirm** -- yes/no decisions (install hook?, overwrite?).
- **Select** -- single-choice (active profile, configuration options).
- **Multi-select** -- multi-choice (which skills to activate).
- **Spinner** -- long operations (indexing, cache building).
- **Note** -- informational summary blocks (what was created).

Every interactive command handles Ctrl+C gracefully at any prompt -- no orphaned state. All commands are idempotent and respond in under 2 seconds for typical repositories.
