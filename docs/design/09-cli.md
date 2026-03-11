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
| usability | `skills init` must complete setup with no more than 3 user prompts |
| reliability | All CLI commands must be idempotent — running twice must be safe |
| performance | CLI commands must respond in under 2s for typical repos |

---

## Layered Profile System

```
~/.skills/                              ← machine-level, developer-owned
  config.yaml                           ← active profile, registered MCPs, global prefs
  profiles/
    personal/
      profile.yaml                      ← developer identity and workflow preferences
      guidelines.md                     ← personal coding standards + workflow cycle
      skills.yaml                       ← which skills are active
    companies/
      <name>/
        profile.yaml                    ← company identity, remote MCP endpoint, metrics
        guidelines.md                   ← shared company standards
        skills.yaml                     ← company-required skills
  cache/
    <lib-name>/                         ← indexed external library
      symbol-map.json
      stack.md
      llmspec.yaml

<repo-root>/                            ← project-level
  .skills/
    project.yaml                        ← active profiles for this repo + overrides
  .llmspec.yaml
  .index/
  CLAUDE.md
  llms.txt
```

**Profile resolution** — when in a repo, the MCP server merges in this order:
1. Personal profile (`~/.skills/profiles/personal/`)
2. Company profile (if `project.yaml` references one)
3. Project overrides (`.skills/project.yaml`)

Each layer extends, never replaces. Company guidelines append to personal. Project overrides append to both.

---

## Config Schemas

### `~/.skills/config.yaml`

```yaml
activeProfile: personal          # Default active profile name
editor: $EDITOR                  # Editor for 'sensei guidelines edit'
mcpRegistrations:                # All registered MCP servers
  - name: repo-index-server
    scope: project               # project | global
    repoPath: /path/to/repo
```

### `~/.skills/profiles/personal/profile.yaml`

```yaml
name: personal
displayName: Jerry                # Used in status output
workflow: idea-feature-design-impl  # Preferred development cycle
preferredLibs:
  - name: rokkit
    path: ~/Developer/rokkit
    cached: true
  - name: kavach
    path: ~/Developer/kavach
    cached: true
```

### `~/.skills/profiles/companies/<name>/profile.yaml`

```yaml
name: acme
displayName: Acme Corp
remoteMcp:
  endpoint: https://mcp.acme.internal
  auth: bearer                   # bearer | none | custom
  tokenEnv: ACME_MCP_TOKEN       # env var holding the token
localCompanionMcp:
  enabled: true                  # Cache remote responses locally
  cacheDir: ~/.skills/mcp-cache/acme/
  ttlSeconds: 3600
metrics:
  enabled: true
  capture: [tokensIn, tokensOut, interactions, toolCalls]
  reportEndpoint: https://metrics.acme.internal
```

### `~/.skills/profiles/personal/skills.yaml` (and company equivalent)

```yaml
active:
  - codebase-indexer
  - content-compression
  - agentic-dev-workflow
  - context-manager
  - doc-drift-detector
  - benchmark-runner
installPath: ~/.claude/skills     # Where to symlink skills
```

### `<repo-root>/.skills/project.yaml`

```yaml
profiles:
  personal: true                  # Always include personal profile
  company: acme                   # Optional company profile reference
overrides:
  stack: [typescript, react]      # Project-specific stack hint
  entryPoints:
    - path: src/index.ts
      role: server entry
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
      "Created: .llmspec.yaml, CLAUDE.md, llms.txt, .index/",
      "Profiles: " + (profiles as string[]).join(", "),
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
1. Detect if .skills/ already exists → warn and suggest 'sensei add' if so
2. Run reindexRepo(cwd)
3. Prompt: which profiles to activate? (list available from ~/.skills/profiles/)
4. Write .skills/project.yaml with selected profiles
5. Prompt: install pre-commit drift hook? (y/n)
6. If yes: run hooksInstall()
7. Print summary: files created, profiles active, hook status
```

### `sensei add`

```
1. Run reindexRepo(cwd) — safe, won't overwrite .llmspec.yaml
2. If no .skills/project.yaml → create with defaults
3. If no CLAUDE.md → create template
4. Report: what was added / what was skipped (already existed)
```

### `sensei upgrade`

```
1. bun update repo-index-server (or pull from git if installed from source)
2. Rebuild MCP server: bun run build
3. Re-run reindexRepo(cwd, { force: true }) for .index/ refresh
4. Regenerate llms.txt and CLAUDE.md (non-destructive merge for CLAUDE.md)
5. Do NOT overwrite .llmspec.yaml
6. Re-symlink skills to ~/.claude/skills/
7. Print changelog of what changed
```

### `sensei status`

Output format:
```
Skills status — /path/to/current/repo
─────────────────────────────────────
Profiles:   personal ✓  |  acme (company) ✓
MCP:        repo-index-server ✓ (registered, running)
            acme-companion ✓ (local cache, remote: https://mcp.acme.internal)
Index:      last updated 2 days ago
Drift:      3 files drifted (run: sensei drift)
Cache:      rokkit ✓ (indexed 5 days ago)  |  kavach ✓ (indexed 1 day ago)
```

### `sensei guidelines`

```
1. Load personal guidelines.md
2. If company profile active: load company guidelines.md
3. Print merged output, each section labelled [personal] or [acme]
```

### `sensei guidelines edit`

```
1. Determine active profile
2. Open ~/.skills/profiles/<profile>/guidelines.md in $EDITOR
3. Wait for editor to close
4. Print: "Guidelines updated."
```

### `sensei cache add <path> [--as <name>]`

```
1. Resolve path
2. Run reindexRepo(path) into ~/.skills/cache/<name>/
3. Register cache entry in ~/.skills/config.yaml
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

## Remote + Companion MCP Architecture

```
Agent
  │
  ├── repo-index-server (local)      ← handles: index, context, drift, guidelines
  │
  └── acme-companion (local)         ← handles: company tools + metrics
        │
        ├── local cache (~/.skills/mcp-cache/acme/)
        │     ↑ cache hit: serve locally (no latency)
        │
        └── acme remote MCP          ← cache miss: fetch from remote, cache response
              https://mcp.acme.internal
```

The companion MCP is a thin proxy registered in `~/.claude/mcp.json` alongside `repo-index-server`. It is configured in `company/profile.yaml` and started/stopped by the CLI.

---

## Package Structure

```
mcp/repo-index-server/
  src/
    index.ts          ← MCP server entry (unchanged)
    cli.ts            ← CLI entry point (new)
    commands/         ← CLI command modules (new)
      init.ts
      add.ts
      upgrade.ts
      status.ts
      profile.ts
      company.ts
      guidelines.ts
      cache.ts
      hooks.ts
      index-cmd.ts    ← wraps reindexRepo for CLI use
      drift-cmd.ts    ← wraps checkDrift for CLI use
    tools/            ← shared: query, reindex, context, drift, generate, benchmark
    types.ts
    index-reader.ts
  package.json        ← "bin": { "sensei": "./dist/cli.js" }
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

Gracefully skips if `skills` isn't installed, so the hook doesn't break for teammates who haven't set up skills yet.
