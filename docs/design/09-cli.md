---
id: cli
type: design
implements:
  - feature: cli
    items: [repo-setup, profile-management, company-profile, context-switching, shared-library-cache, migration, pre-commit-hook, guidelines]
---

# CLI

## Overview

`sensei` is a TypeScript CLI installed globally via `bun add -g sensei` or `npx sensei`. It shares the same codebase as the MCP server — same modules, two entry points (`cli.ts` for the CLI, `index.ts` for the MCP server). It manages three layers: developer profile, company profile, and project/repo configuration.

## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| usability | `sensei init` must complete setup with no more than 3 user prompts |
| reliability | All CLI commands must be idempotent — running twice must be safe |
| performance | CLI commands must respond in under 2s for typical repos |

---

## Layered Profile System

```
~/.config/sensei/                   ← global developer config
  config.yaml                       ← active backend, auth tokens, global prefs

<repo-root>/                        ← project-level
  .sensei/
    config.yaml                     ← project config (custom_libs, ranking strategy)
  CLAUDE.md
  .sensei/llmspec.yaml
  .sensei/llms.txt
```

Authentication and team isolation is a Phase 9 feature. Phase 1 uses simple global config at `~/.config/sensei/config.yaml`.

---

## Config Schemas

### `~/.config/sensei/config.yaml`

```yaml
backend: supabase                # Active storage backend
supabaseUrl: https://...         # Supabase project URL
supabaseKey: ...                 # Supabase anon/service key
editor: $EDITOR                  # Editor for 'sensei guidelines edit'
```

### `<repo-root>/.sensei/config.yaml`

```yaml
custom_libs:                     # Library intelligence hints
  - name: rokkit
    path: ~/Developer/rokkit
rankingStrategy: default         # Symbol ranking strategy for this project
```

---

## Prompts: Clack

All interactive prompts use [`@clack/prompts`](https://github.com/natemoo-re/clack). No custom prompt logic.

**Primitives used:**

| Clack primitive | Used for |
|---|---|
| `intro` / `outro` | Session start/end with styled header |
| `text` | Free-text input (project name, MCP endpoint URL) |
| `confirm` | Yes/no decisions (install hook?, overwrite?) |
| `select` | Single-choice (active profile, company name) |
| `multiselect` | Multi-choice (which skills to activate) |
| `spinner` | Long operations (indexing, building MCP server) |
| `note` | Informational summary blocks (what was created) |
| `log.success` / `log.warn` / `log.error` | Status messages |
| `isCancel` | Handle Ctrl+C gracefully at any prompt |

**Pattern for all interactive commands:**

```typescript
import { intro, outro, select, confirm, multiselect, spinner, note, isCancel, cancel } from "@clack/prompts";

export async function init() {
  intro("sensei init");

  const profiles = await multiselect({
    message: "Which profiles to activate for this repo?",
    options: [
      { value: "personal", label: "Personal", hint: "your standards and workflow" },
      { value: "acme", label: "Acme Corp", hint: "company guidelines + remote MCP" },
    ],
    required: true,
  });
  if (isCancel(profiles)) { cancel("Setup cancelled."); process.exit(0); }

  const installHook = await confirm({ message: "Install pre-commit drift hook?" });
  if (isCancel(installHook)) { cancel("Setup cancelled."); process.exit(0); }

  const s = spinner();
  s.start("Indexing repo...");
  await reindexRepo(cwd);
  s.stop("Repo indexed.");

  note(
    [
      "Created: .sensei/llmspec.yaml, .sensei/llms.txt, CLAUDE.md",
      installHook ? "Hook: pre-commit drift check installed" : "Hook: skipped",
    ].join("\n"),
    "Setup complete"
  );

  outro("Run sensei status anytime to check your setup.");
}
```

**Cancellation contract:** Every command wraps prompts in `isCancel` checks and exits cleanly with `cancel()`. No orphaned state.

---

## CLI Entry Point

`src/cli.ts` — thin command dispatcher using `node:util` parseArgs (no heavy frameworks):

```typescript
#!/usr/bin/env node
import { parseArgs } from "node:util";
import { init } from "./commands/init.js";
import { add } from "./commands/add.js";
import { upgrade } from "./commands/upgrade.js";
import { status } from "./commands/status.js";
import { profileCreate, profileEdit, profileList, profileUse } from "./commands/profile.js";
import { companyCreate, companyEdit, companyRegisterMcp } from "./commands/company.js";
import { guidelines, guidelinesEdit, guidelinesShow } from "./commands/guidelines.js";
import { cacheAdd, cacheList, cacheUpdate } from "./commands/cache.js";
import { hooksInstall } from "./commands/hooks.js";
import { index } from "./commands/index-cmd.js";
import { drift } from "./commands/drift-cmd.js";

const [,, cmd, ...args] = process.argv;
// dispatch to command modules
```

---

## Command Modules

Each command lives in `src/commands/<name>.ts` and imports from `src/tools/` (shared with MCP server).

### `sensei init`

```
1. Detect if .sensei/ already exists → warn and suggest 'sensei add' if so
2. Run reindexRepo(cwd)
3. Write .sensei/config.yaml with project defaults
4. Prompt: install pre-commit drift hook? (y/n)
5. If yes: run hooksInstall()
6. Print summary: files created, hook status
```

### `sensei add`

```
1. Run reindexRepo(cwd) — safe, won't overwrite .sensei/llmspec.yaml
2. If no .sensei/config.yaml → create with defaults
3. If no CLAUDE.md → create template
4. Report: what was added / what was skipped (already existed)
```

### `sensei upgrade`

```
1. bun update sensei (or pull from git if installed from source)
2. Rebuild: bun run build
3. Re-run reindexRepo(cwd, { force: true }) to refresh Supabase symbols
4. Regenerate .sensei/llms.txt and CLAUDE.md (non-destructive merge for CLAUDE.md)
5. Do NOT overwrite .sensei/llmspec.yaml
6. Print changelog of what changed
```

### `sensei status`

Output format:
```
sensei status — /path/to/current/repo
──────────────────────────────────────
Config:     ~/.config/sensei/config.yaml ✓
MCP:        packages/server ✓ (registered, running)
Index:      last updated 2 days ago (sensei.symbols)
Drift:      3 files drifted (run: sensei drift)
```

### `sensei guidelines`

```
1. Load personal guidelines.md
2. If company profile active: load company guidelines.md
3. Print merged output, each section labelled [personal] or [acme]
```

### `sensei guidelines edit`

```
1. Open ~/.config/sensei/guidelines.md in $EDITOR
2. Wait for editor to close
3. Print: "Guidelines updated."
```

### `sensei cache add <path> [--as <name>]`

Library intelligence is handled via `custom_libs` in `.sensei/config.yaml` (see feature doc 05-library-intelligence). This command registers a library path and triggers indexing into Supabase.

```
1. Resolve path
2. Run reindexRepo(path) — stores symbols in sensei.symbols
3. Add entry to .sensei/config.yaml under custom_libs
4. Print: "Cached <name> (N files indexed)"
```

### `sensei hooks install [--drift]`

```
1. Write .git/hooks/pre-commit:
   #!/bin/sh
   sensei drift --fail-on-drift
2. chmod +x .git/hooks/pre-commit
3. Print: "Pre-commit drift hook installed."
```

---

## MCP Server Extensions

New MCP tools to support CLI-level features:

| Tool | Purpose |
|---|---|
| `get_guidelines(section?)` | Return active merged guidelines or a named section |
| `get_profile(scope?)` | Return active profile(s): personal, company, or both |
| `query_cache(lib, path, level)` | Query a cached external library at a resolution level |
| `get_company_metrics_config()` | Return metrics capture config if company profile active |

---

## MCP Server

The MCP server lives at `packages/server/`. It is registered in `~/.claude/mcp.json` and handles: index, context, drift, guidelines tools. No companion proxy or remote MCP in Phase 1 — team isolation is a Phase 9 feature.

---

## Package Structure

```
packages/cli/         ← CLI entry point
  src/
    cli.ts            ← CLI entry point
    commands/         ← CLI command modules
      init.ts
      add.ts
      upgrade.ts
      status.ts
      guidelines.ts
      cache.ts
      hooks.ts
      index-cmd.ts    ← wraps reindexRepo for CLI use
      drift-cmd.ts    ← wraps checkDrift for CLI use
  package.json        ← "bin": { "sensei": "./dist/cli.js" }

packages/server/      ← MCP server
  src/
    index.ts          ← MCP server entry
    tools/            ← shared: query, reindex, context, drift, generate, benchmark

packages/engine/      ← inference; used directly (no HTTP server)
```

Adding `"bin"` to `package.json` makes `sensei` available as a global command after `bun add -g sensei`.

**Dependencies added for CLI:**

```json
{
  "dependencies": {
    "@clack/prompts": "^0.9.0"
  }
}
```

All interactive prompts go through `@clack/prompts` — no readline, no inquirer, no custom prompt code. See the Clack section above for the full primitive set.

---

## Guidelines File Format

`guidelines.md` is plain markdown with named sections. The MCP `get_guidelines(section)` tool extracts sections by H2 heading.

```markdown
# Guidelines

## Workflow
idea → features (docs/features/) → design (docs/design/) → implementation plan → execute

## Coding Standards
- Repository pattern for all DB access
- Result<T,E> instead of throw for recoverable errors
- L0/L1/L2 resolution before loading full source

## Commit Style
feat: / fix: / chore: / docs: prefixes
Co-authored-by on AI-assisted commits

## Testing
TDD: write failing test first, then minimal implementation
```

---

## Pre-Commit Hook Template

Written to `.git/hooks/pre-commit` by `sensei hooks install --drift`:

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

Gracefully skips if `sensei` isn't installed, so the hook doesn't break for teammates who haven't set up sensei yet.
