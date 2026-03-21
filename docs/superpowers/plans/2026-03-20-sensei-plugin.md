# Sensei Plugin Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create a Claude Code plugin at `plugin/` in the sensei repo that packages all skills and commands, installs via `bun run plugin:install`, and auto-discovers skills at session start.

**Architecture:** Plugin lives in `plugin/` directory, installed by copying to `~/.claude/plugins/cache/sensei/local/1.0.0/` and registering in `~/.claude/plugins/installed_plugins.json`. A SessionStart hook reads per-project `sensei.config.json` to inject opt-in skill context. Default skills fire globally; opt-in skills fire only in configured projects.

**Tech Stack:** TypeScript (Bun), Bash (hook scripts), Markdown (skills/commands)

**Spec:** `docs/superpowers/specs/2026-03-20-sensei-plugin-design.md`

---

## Chunk 1: Plugin Scaffold + Install Script

### Task 1: Create plugin directory structure

**Files:**
- Create: `plugin/.claude-plugin/plugin.json`
- Create: `plugin/skills/` (directory)
- Create: `plugin/commands/` (directory)
- Create: `plugin/hooks/` (directory)
- Create: `plugin/scripts/` (directory)

- [ ] **Step 1: Create the directory tree**

```bash
mkdir -p plugin/.claude-plugin
mkdir -p plugin/skills
mkdir -p plugin/commands
mkdir -p plugin/hooks
mkdir -p plugin/scripts
```

- [ ] **Step 2: Write plugin.json**

Create `plugin/.claude-plugin/plugin.json`:

```json
{
  "name": "sensei",
  "version": "1.0.0",
  "description": "Dev workflow skills and commands — cross-project guardrails plus project-specific opt-ins",
  "author": {
    "name": "Jerry"
  },
  "skills": "./skills/",
  "commands": "./commands/",
  "hooks": "./hooks/hooks.json"
}
```

- [ ] **Step 3: Verify structure**

```bash
find plugin/ -type f | sort
```
Expected: `.claude-plugin/plugin.json` only (skills/commands/hooks dirs are empty for now).

- [ ] **Step 4: Commit**

```bash
git add plugin/
git commit -m "chore(plugin): scaffold plugin directory structure"
```

---

### Task 2: Write install + uninstall scripts

**Files:**
- Create: `plugin/scripts/install.ts`
- Create: `plugin/scripts/uninstall.ts`
- Modify: `package.json` (root) — add `plugin:install` and `plugin:uninstall` scripts

The install script copies `plugin/` to `~/.claude/plugins/cache/sensei/local/1.0.0/` and upserts the plugin entry in `~/.claude/plugins/installed_plugins.json`. Uninstall reverses this.

The project slug format (used throughout the plugin) is derived from the absolute project path by replacing `/` with `-` and stripping the leading `-`. Example: `/Users/Jerry/Developer/sensei` → `Users-Jerry-Developer-sensei`. Verify by running: `ls ~/.claude/projects/` — you'll see slugs like `-Users-Jerry-Developer-sensei` (note leading `-`).

- [ ] **Step 1: Write install.ts**

Create `plugin/scripts/install.ts`:

```typescript
import { cp, mkdir, readFile, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { join, resolve, dirname } from "node:path";
import { homedir } from "node:os";
import { fileURLToPath } from "node:url";

const PLUGIN_NAME = "sensei";
const PLUGIN_VERSION = "1.0.0";
const PLUGIN_KEY = `${PLUGIN_NAME}@local`;

const __dirname = dirname(fileURLToPath(import.meta.url));
const pluginSrc = resolve(__dirname, "..");
const claudeDir = join(homedir(), ".claude");
const pluginsDir = join(claudeDir, "plugins");
const installPath = join(pluginsDir, "cache", PLUGIN_NAME, "local", PLUGIN_VERSION);
const installedPluginsPath = join(pluginsDir, "installed_plugins.json");

async function install() {
  // 1. Copy plugin directory
  await mkdir(installPath, { recursive: true });
  await cp(pluginSrc, installPath, { recursive: true, force: true });
  console.log(`✓ Copied plugin to ${installPath}`);

  // 2. Read existing installed_plugins.json
  let registry: { version: number; plugins: Record<string, unknown[]> } = {
    version: 2,
    plugins: {},
  };
  if (existsSync(installedPluginsPath)) {
    const raw = await readFile(installedPluginsPath, "utf-8");
    registry = JSON.parse(raw);
  }

  // 3. Upsert plugin entry
  registry.plugins[PLUGIN_KEY] = [
    {
      scope: "user",
      installPath,
      version: PLUGIN_VERSION,
      installedAt: new Date().toISOString(),
      lastUpdated: new Date().toISOString(),
    },
  ];

  // 4. Write back
  await writeFile(installedPluginsPath, JSON.stringify(registry, null, 2));
  console.log(`✓ Registered ${PLUGIN_KEY} in installed_plugins.json`);

  // 5. Summary
  const skillsDir = join(installPath, "skills");
  const commandsDir = join(installPath, "commands");
  const { readdirSync } = await import("node:fs");
  const skills = existsSync(skillsDir) ? readdirSync(skillsDir) : [];
  const commands = existsSync(commandsDir) ? readdirSync(commandsDir).filter(f => f.endsWith(".md")) : [];
  console.log(`\nSensei plugin installed:`);
  console.log(`  Skills:   ${skills.length} (${skills.join(", ")})`);
  console.log(`  Commands: ${commands.length} (${commands.map(c => "/" + c.replace(".md", "")).join(", ")})`);
  console.log(`\nRestart Claude Code to activate.`);
}

install().catch((err) => {
  console.error("Install failed:", err.message);
  process.exit(1);
});
```

- [ ] **Step 2: Write uninstall.ts**

Create `plugin/scripts/uninstall.ts`:

```typescript
import { rm, readFile, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { homedir } from "node:os";

const PLUGIN_NAME = "sensei";
const PLUGIN_KEY = `${PLUGIN_NAME}@local`;

const claudeDir = join(homedir(), ".claude");
const pluginsDir = join(claudeDir, "plugins");
const installPath = join(pluginsDir, "cache", PLUGIN_NAME, "local");
const installedPluginsPath = join(pluginsDir, "installed_plugins.json");

async function uninstall() {
  // Remove installed files
  if (existsSync(installPath)) {
    await rm(installPath, { recursive: true, force: true });
    console.log(`✓ Removed ${installPath}`);
  } else {
    console.log(`Nothing to remove at ${installPath}`);
  }

  // Remove from registry
  if (existsSync(installedPluginsPath)) {
    const raw = await readFile(installedPluginsPath, "utf-8");
    const registry = JSON.parse(raw);
    if (registry.plugins[PLUGIN_KEY]) {
      delete registry.plugins[PLUGIN_KEY];
      await writeFile(installedPluginsPath, JSON.stringify(registry, null, 2));
      console.log(`✓ Deregistered ${PLUGIN_KEY}`);
    }
  }

  console.log(`\nSensei plugin uninstalled. Restart Claude Code to deactivate.`);
}

uninstall().catch((err) => {
  console.error("Uninstall failed:", err.message);
  process.exit(1);
});
```

- [ ] **Step 3: Add scripts to root package.json**

In the root `package.json`, find the `"scripts"` object and add:

```json
"plugin:install": "bun run plugin/scripts/install.ts",
"plugin:uninstall": "bun run plugin/scripts/uninstall.ts"
```

- [ ] **Step 4: Run install and verify**

```bash
bun run plugin:install
```

Expected output:
```
✓ Copied plugin to /Users/<you>/.claude/plugins/cache/sensei/local/1.0.0
✓ Registered sensei@local in installed_plugins.json
Sensei plugin installed:
  Skills:   0 ()
  Commands: 0 ()
Restart Claude Code to activate.
```

Then verify:
```bash
cat ~/.claude/plugins/installed_plugins.json | grep -A 6 '"sensei@local"'
```

- [ ] **Step 5: Commit**

```bash
git add plugin/scripts/ package.json
git commit -m "feat(plugin): add install/uninstall scripts"
```

---

## Chunk 2: Default Skills

Four skills, always active in all projects. Each skill goes in `plugin/skills/<name>/SKILL.md`.

The `name` and `description` frontmatter fields are what Claude sees in the skills list — make them precise trigger conditions. Bad description = skill never fires.

### Task 3: working-smarter skill (merges 3 existing)

**Files:**
- Create: `plugin/skills/working-smarter/SKILL.md`
- Source material:
  - `~/.claude/skills/working-smarter.md`
  - `sensei/skills/building-app-mockups/SKILL.md`
  - `sensei/skills/zero-errors-policy/SKILL.md`

- [ ] **Step 1: Read all three source skills**

```bash
cat ~/.claude/skills/working-smarter.md
cat /Users/Jerry/Developer/sensei/skills/building-app-mockups/SKILL.md
cat /Users/Jerry/Developer/sensei/skills/zero-errors-policy/SKILL.md
```

- [ ] **Step 2: Write merged SKILL.md**

Create `plugin/skills/working-smarter/SKILL.md` merging all three. The skill must cover:
1. **Before any new feature:** commit all uncommitted work first
2. **Mockups:** always build in the target framework (`/mockups/a`, `/mockups/b` routes), never standalone HTML
3. **Zero-errors:** run `bun run --filter '*' test && bunx tsc --noEmit` before AND after any implementation task

Frontmatter:
```yaml
---
name: working-smarter
description: Use when designing UI mockups, building new features, or starting/completing any implementation task — enforces commit-first discipline, framework-native mockups (no standalone HTML), and zero-errors checkpoints before and after coding.
---
```

- [ ] **Step 3: Run install to push skill live**

```bash
bun run plugin:install
```

- [ ] **Step 4: Verify skill appears in Claude's skills list**

Start a new Claude Code session and check that `working-smarter` appears in the available skills list shown in session-start system reminder.

- [ ] **Step 5: Commit**

```bash
git add plugin/skills/working-smarter/
git commit -m "feat(plugin): add working-smarter skill (merges mockups + zero-errors)"
```

---

### Task 4: context-efficiency skill (merges 2 existing)

**Files:**
- Create: `plugin/skills/context-efficiency/SKILL.md`
- Source material:
  - `sensei/skills/managing-context/SKILL.md`
  - `sensei/skills/compressing-content/SKILL.md`

- [ ] **Step 1: Read both source skills**

```bash
cat /Users/Jerry/Developer/sensei/skills/managing-context/SKILL.md
cat /Users/Jerry/Developer/sensei/skills/compressing-content/SKILL.md
```

- [ ] **Step 2: Write merged SKILL.md**

Create `plugin/skills/context-efficiency/SKILL.md`. Coverage:
1. Call `recommend_next(task)` before loading any code to get minimal scope
2. Choose resolution level (L0 signature → L3 full source) based on task need
3. Never load a file speculatively — only what the task requires

Frontmatter:
```yaml
---
name: context-efficiency
description: Use before loading any code into a session — calls recommend_next(task) to get the minimal file scope and resolution level, preventing token bloat without sacrificing accuracy.
---
```

- [ ] **Step 3: Install + commit**

```bash
bun run plugin:install
git add plugin/skills/context-efficiency/
git commit -m "feat(plugin): add context-efficiency skill (merges managing-context + compressing-content)"
```

---

### Task 5: decomposing-broad-tasks skill

**Files:**
- Create: `plugin/skills/decomposing-broad-tasks/SKILL.md`
- Source: `sensei/skills/decomposing-broad-tasks/SKILL.md`

- [ ] **Step 1: Copy and review source skill**

```bash
cat /Users/Jerry/Developer/sensei/skills/decomposing-broad-tasks/SKILL.md
```

- [ ] **Step 2: Write SKILL.md**

Copy content into `plugin/skills/decomposing-broad-tasks/SKILL.md`. Keep frontmatter unchanged — the trigger is already precise:
```yaml
---
name: decomposing-broad-tasks
description: Use when a request touches 5+ files or mentions "all", "every", "refactor all", "update all", "clean up all", or "audit all" — or when about to Glob a pattern that will return 5+ results and read most of them.
---
```

- [ ] **Step 3: Install + commit**

```bash
bun run plugin:install
git add plugin/skills/decomposing-broad-tasks/
git commit -m "feat(plugin): add decomposing-broad-tasks skill"
```

---

### Task 6: pattern-based-development skill

**Files:**
- Create: `plugin/skills/pattern-based-development/SKILL.md`
- Source: `sensei/skills/pattern-based-development/SKILL.md`

- [ ] **Step 1: Read source skill**

```bash
cat /Users/Jerry/Developer/sensei/skills/pattern-based-development/SKILL.md
```

- [ ] **Step 2: Write SKILL.md**

Copy into `plugin/skills/pattern-based-development/SKILL.md`. Update description to make it universally applicable (remove sensei-specific PATTERNS.md references if present — use generic "project's PATTERNS.md").

Frontmatter:
```yaml
---
name: pattern-based-development
description: Use before implementing any new feature, component, module, or integration — checks PATTERNS.md for an applicable recipe before writing new code. Prevents re-inventing structure that already exists in this codebase.
---
```

- [ ] **Step 3: Install + commit**

```bash
bun run plugin:install
git add plugin/skills/pattern-based-development/
git commit -m "feat(plugin): add pattern-based-development as default skill"
```

---

## Chunk 3: Opt-in Skills + Hooks

### Task 7: session-management opt-in skill (merges 2 existing)

**Files:**
- Create: `plugin/skills/session-management/SKILL.md`
- Source material:
  - `sensei/skills/managing-project-sessions/SKILL.md`
  - `sensei/skills/running-agentic-sessions/SKILL.md`

- [ ] **Step 1: Read both source skills**

```bash
cat /Users/Jerry/Developer/sensei/skills/managing-project-sessions/SKILL.md
cat /Users/Jerry/Developer/sensei/skills/running-agentic-sessions/SKILL.md
```

- [ ] **Step 2: Write merged SKILL.md**

Create `plugin/skills/session-management/SKILL.md`. Coverage:
1. `get_session_context()` — always call at session start when MCP tools available
2. `take_snapshot(progress_summary)` — at key decision points
3. `recommend_next(task)` — before loading any code slice
4. `checkpoint(outcome, summary)` — end of every task

Frontmatter:
```yaml
---
name: session-management
description: Use at the start of every session when sensei MCP tools are available — calls get_session_context() to resume from last checkpoint, surface open decisions, and orient without re-reading git log or files. Also governs take_snapshot() and checkpoint() usage throughout the session.
---
```

- [ ] **Step 3: Install + commit**

```bash
bun run plugin:install
git add plugin/skills/session-management/
git commit -m "feat(plugin): add session-management opt-in skill"
```

---

### Task 8: codebase-indexing opt-in skill (merges 2 existing)

**Files:**
- Create: `plugin/skills/codebase-indexing/SKILL.md`
- Source material:
  - `sensei/skills/indexing-codebase/SKILL.md`
  - `sensei/skills/populating-llmspec/SKILL.md`

- [ ] **Step 1: Read sources + write merged SKILL.md**

```bash
cat /Users/Jerry/Developer/sensei/skills/indexing-codebase/SKILL.md
cat /Users/Jerry/Developer/sensei/skills/populating-llmspec/SKILL.md
```

Frontmatter for `plugin/skills/codebase-indexing/SKILL.md`:
```yaml
---
name: codebase-indexing
description: Use when first working on a repo, after a major refactor, or when llmspec.yaml has TODO placeholders — runs the sensei indexer to produce llmspec.yaml and symbol-map, then populates any empty fields.
---
```

- [ ] **Step 2: Install + commit**

```bash
bun run plugin:install
git add plugin/skills/codebase-indexing/
git commit -m "feat(plugin): add codebase-indexing opt-in skill"
```

---

### Task 9: Remaining opt-in skills (4 skills)

**Files:**
- Create: `plugin/skills/identifying-patterns/SKILL.md`
- Create: `plugin/skills/guiding-doc-creation/SKILL.md`
- Create: `plugin/skills/running-benchmarks/SKILL.md`
- Create: `plugin/skills/auditing-skill-descriptions/SKILL.md`

Sources in `sensei/skills/<name>/SKILL.md` for each. Copy content as-is — these don't need merging.

- [ ] **Step 1: Copy all four skills**

```bash
for skill in identifying-patterns guiding-doc-creation running-benchmarks auditing-skill-descriptions; do
  mkdir -p plugin/skills/$skill
  cp /Users/Jerry/Developer/sensei/skills/$skill/SKILL.md plugin/skills/$skill/SKILL.md
done
```

- [ ] **Step 2: Verify all four copied correctly**

```bash
for skill in identifying-patterns guiding-doc-creation running-benchmarks auditing-skill-descriptions; do
  echo "=== $skill ===" && head -5 plugin/skills/$skill/SKILL.md
done
```

- [ ] **Step 3: Install + commit**

```bash
bun run plugin:install
git add plugin/skills/
git commit -m "feat(plugin): add remaining opt-in skills (4 skills)"
```

---

### Task 10: SessionStart hook

The hook fires at session start, reads `~/.claude/projects/<slug>/sensei.config.json`, and if opt-in skills are configured, outputs them as `additionalContext` so Claude knows they're active.

Project slug derivation: take the absolute project root path, replace each `/` with `-`. The path `CLAUDE_PROJECT_ROOT` env var (or `PWD` as fallback) gives the project root. Example: `/Users/Jerry/Developer/sensei` → `-Users-Jerry-Developer-sensei`.

**Files:**
- Create: `plugin/hooks/hooks.json`
- Create: `plugin/hooks/run-hook.cmd` (copy from superpowers — cross-platform wrapper)
- Create: `plugin/hooks/session-start` (bash script)

- [ ] **Step 1: Copy run-hook.cmd from superpowers**

```bash
cp ~/.claude/plugins/cache/claude-plugins-official/superpowers/5.0.0/hooks/run-hook.cmd \
   plugin/hooks/run-hook.cmd
```

- [ ] **Step 2: Write hooks.json**

Create `plugin/hooks/hooks.json`:

```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "startup|resume|clear|compact",
        "hooks": [
          {
            "type": "command",
            "command": "'${CLAUDE_PLUGIN_ROOT}/hooks/run-hook.cmd' session-start",
            "async": false
          }
        ]
      }
    ]
  }
}
```

- [ ] **Step 3: Write session-start bash script**

Create `plugin/hooks/session-start`:

```bash
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"

# Derive project slug from project root path
PROJECT_ROOT="${CLAUDE_PROJECT_ROOT:-${PWD}}"
# Replace / with - to match ~/.claude/projects/ slug format
SLUG="${PROJECT_ROOT//\//-}"
CONFIG_FILE="${HOME}/.claude/projects/${SLUG}/sensei.config.json"

escape_for_json() {
  local s="$1"
  s="${s//\\/\\\\}"
  s="${s//\"/\\\"}"
  s="${s//$'\n'/\\n}"
  s="${s//$'\r'/\\r}"
  s="${s//$'\t'/\\t}"
  printf '%s' "$s"
}

# If no project config, output empty and exit
if [ ! -f "$CONFIG_FILE" ]; then
  cat <<EOF
{
  "additional_context": "",
  "hookSpecificOutput": {
    "hookEventName": "SessionStart",
    "additionalContext": ""
  }
}
EOF
  exit 0
fi

# Read opt-in skills list
SKILLS_JSON=$(cat "$CONFIG_FILE" 2>/dev/null || echo '{"skills":[]}')
SKILLS=$(echo "$SKILLS_JSON" | python3 -c "
import json, sys
d = json.load(sys.stdin)
skills = d.get('skills', [])
print(', '.join(skills))
" 2>/dev/null || echo "")

if [ -z "$SKILLS" ]; then
  cat <<EOF
{
  "additional_context": "",
  "hookSpecificOutput": {
    "hookEventName": "SessionStart",
    "additionalContext": ""
  }
}
EOF
  exit 0
fi

CONTEXT="<sensei-opt-in-skills>
The following opt-in skills are active for this project: ${SKILLS}
Use the Skill tool to invoke them when their trigger conditions are met.
</sensei-opt-in-skills>"

ESCAPED=$(escape_for_json "$CONTEXT")

cat <<EOF
{
  "additional_context": "${ESCAPED}",
  "hookSpecificOutput": {
    "hookEventName": "SessionStart",
    "additionalContext": "${ESCAPED}"
  }
}
EOF
exit 0
```

- [ ] **Step 4: Make session-start executable**

```bash
chmod +x plugin/hooks/session-start
```

- [ ] **Step 5: Test hook manually**

```bash
# Test with no config (should output empty context)
CLAUDE_PROJECT_ROOT=/tmp/nonexistent bash plugin/hooks/session-start

# Create a test config and test with opt-in skills
mkdir -p ~/.claude/projects/-tmp-test-project
echo '{"skills":["session-management","codebase-indexing"]}' > ~/.claude/projects/-tmp-test-project/sensei.config.json
CLAUDE_PROJECT_ROOT=/tmp/test-project bash plugin/hooks/session-start

# Cleanup
rm -rf ~/.claude/projects/-tmp-test-project
```

Expected for second test: JSON with `additionalContext` containing `session-management, codebase-indexing`.

- [ ] **Step 6: Install + commit**

```bash
bun run plugin:install
git add plugin/hooks/
git commit -m "feat(plugin): add SessionStart hook for opt-in skill injection"
```

---

## Chunk 4: Commands

Each command is a Markdown file in `plugin/commands/`. Format:

```markdown
---
description: <one line shown in /help>
argument-hint: <optional hint for args>
---

# Command content here
```

### Task 11: Session commands (session, checkpoint, backlog)

**Files:**
- Create: `plugin/commands/session.md`
- Create: `plugin/commands/checkpoint.md`
- Create: `plugin/commands/backlog.md`

- [ ] **Step 1: Write session.md**

```markdown
---
description: Resume session — calls get_session_context() and surfaces open decisions
---

Call `get_session_context(task_description="session startup")`.

Then:
1. Review any open decisions or interrupted work it returns
2. Call `recommend_next(task)` if you're about to start a specific task
3. Report back to the user: what's in progress, what's pending, any blockers
```

- [ ] **Step 2: Write checkpoint.md**

```markdown
---
description: Snapshot current progress for interruption recovery
argument-hint: Brief description of current state
---

Call `take_snapshot(progress_summary="$ARGUMENTS")`.

If $ARGUMENTS is empty, ask the user: "What should I record as the current state?"

After snapshotting, confirm: "Checkpoint saved — you can safely pause and resume from here."
```

- [ ] **Step 3: Write backlog.md**

```markdown
---
description: List open tasks, decisions, and pending questions from the session store
---

Call `get_session_context(task_description="backlog review")`.

From the response, extract and display:
1. **Open decisions** — things awaiting a choice
2. **Pending tasks** — work in progress or queued
3. **Blocked items** — anything waiting on external input
4. **Questions** — unresolved questions from previous sessions

Format as a prioritized list. If the session store is empty, say so.
```

- [ ] **Step 4: Commit**

```bash
git add plugin/commands/session.md plugin/commands/checkpoint.md plugin/commands/backlog.md
git commit -m "feat(plugin): add session, checkpoint, backlog commands"
```

---

### Task 12: Dev discipline commands (mockup, commit)

**Files:**
- Create: `plugin/commands/mockup.md`
- Create: `plugin/commands/commit.md`

- [ ] **Step 1: Write mockup.md**

```markdown
---
description: Start a mockup — enforces framework-native build, commits first
argument-hint: What you want to mockup
---

Before building anything:

1. Check for uncommitted work: `git status`
2. If dirty — commit it first. No exceptions.
3. Confirm the target framework (SvelteKit, React, Next.js, etc.)

Then invoke the `working-smarter` skill to guide the mockup build:
- Build at a real route in the app (e.g. `/mockups/a`, `/mockups/b`)
- Use the project's actual components, tokens, and layout primitives
- Never create a standalone HTML file

If $ARGUMENTS is provided, use it as the mockup description to start immediately.
```

- [ ] **Step 2: Write commit.md**

```markdown
---
description: Run zero-errors checks then commit
argument-hint: Optional commit message
---

## Step 1: Zero-errors checkpoint

Run the full check:
```bash
bun run --filter '*' test && bunx tsc --noEmit
```

If any errors: **stop**. Do not commit. Fix all errors first, then run again.

## Step 2: Commit

Only proceed once the above passes with zero errors.

- Review staged changes: `git diff --staged`
- Stage relevant files if not already staged
- Commit with a clear message

If $ARGUMENTS is provided, use it as the commit message. Otherwise write one based on the staged changes.
```

- [ ] **Step 3: Commit**

```bash
git add plugin/commands/mockup.md plugin/commands/commit.md
git commit -m "feat(plugin): add mockup and commit commands"
```

---

### Task 13: Pattern commands (pattern-extract, pattern-use)

**Files:**
- Create: `plugin/commands/pattern-extract.md`
- Create: `plugin/commands/pattern-use.md`

- [ ] **Step 1: Write pattern-extract.md**

```markdown
---
description: Extract a reusable pattern from existing code and write it to PATTERNS.md
argument-hint: Description of the pattern to extract
---

## Extracting a Pattern

Goal: document a recurring implementation structure so future features can follow it instead of re-inventing it.

1. Identify the code to extract from (ask the user if $ARGUMENTS is vague)
2. Read the relevant files and identify:
   - The repeating structure (what files, what shape)
   - The key decision points (what varies vs what's fixed)
   - The naming conventions
3. Write the pattern to `PATTERNS.md` using this format:

```markdown
## Pattern: <Name>

**When to use:** <trigger condition>

**Structure:**
- File 1: `<path pattern>` — <responsibility>
- File 2: `<path pattern>` — <responsibility>

**Key conventions:**
- <convention 1>
- <convention 2>

**Example:**
<minimal concrete example>
```

4. Confirm the pattern was added and show the user the written entry.
```

- [ ] **Step 2: Write pattern-use.md**

```markdown
---
description: Look up a pattern by name and apply it to the current task
argument-hint: Pattern name or description
---

1. Read `PATTERNS.md`
2. Find the pattern matching $ARGUMENTS (fuzzy match if needed)
3. If no match: list available patterns and ask the user which to use
4. Show the user the matched pattern
5. Apply it to the current task:
   - Use the exact file structure and naming conventions from the pattern
   - Highlight any decision points where the user needs to provide values
   - Do not deviate from the pattern without flagging it
```

- [ ] **Step 3: Commit**

```bash
git add plugin/commands/pattern-extract.md plugin/commands/pattern-use.md
git commit -m "feat(plugin): add pattern-extract and pattern-use commands"
```

---

### Task 14: Reverse-engineering commands (product, feature, audit)

These commands drive the workflow from `docs/reverse-engineer.md`. The command files reference that doc as the skill source.

**Files:**
- Create: `plugin/commands/product.md`
- Create: `plugin/commands/feature.md`
- Create: `plugin/commands/audit.md`

- [ ] **Step 1: Write product.md**

```markdown
---
description: Reverse-engineer the full product — generates openspec/product/ docs
argument-hint: Optional root path (defaults to .)
---

Read `docs/reverse-engineer.md` in full, then execute it with:

```
mode=product root=$ARGUMENTS
```

If $ARGUMENTS is empty, use `root=.`

Follow the workflow exactly as specified in reverse-engineer.md:
- Auto-detect all stacks
- Generate all product-level docs under `openspec/product/`
- Apply backlog ID schema
- Enforce overwrite protection
```

- [ ] **Step 2: Write feature.md**

```markdown
---
description: Deep-dive a feature — generates openspec/specs/<capability>/ docs
argument-hint: Capability name (e.g. auth, payments)
---

Read `docs/reverse-engineer.md` in full, then execute it with:

```
mode=feature capability=$ARGUMENTS
```

If $ARGUMENTS is empty, run interactive feature selection (requires `openspec/product/features.md` to exist — run `/product` first if it doesn't).

Follow the workflow exactly as specified in reverse-engineer.md:
- Generate proposal.md, spec.md, design.md, api.md, flow-diagram.md, nfr.md
- Apply backlog ID schema under `BL-<CAPABILITY>-*`
- Enforce overwrite protection
```

- [ ] **Step 3: Write audit.md**

```markdown
---
description: Audit a capability for OWASP, NFR, and code quality issues
argument-hint: Capability name (omit to audit everything)
---

Read `docs/reverse-engineer.md` in full, then execute it with:

```
mode=audit capability=$ARGUMENTS
```

If $ARGUMENTS is empty, audit ALL capabilities under `openspec/specs/` plus `specs/common/` and all DB docs.

Follow the workflow exactly as specified in reverse-engineer.md:
- OWASP 2021 security checks
- NFR coverage across 6 dimensions
- Code quality health score
- Apply backlog ID schema under `BL-<SCOPE>-*`
```

- [ ] **Step 4: Commit**

```bash
git add plugin/commands/product.md plugin/commands/feature.md plugin/commands/audit.md
git commit -m "feat(plugin): add product, feature, audit commands from reverse-engineer workflow"
```

---

### Task 15: Config management commands (enable, disable)

**Files:**
- Create: `plugin/commands/enable.md`
- Create: `plugin/commands/disable.md`

The project slug is derived from the current working directory: replace all `/` with `-`.

Available opt-in skills: `session-management`, `codebase-indexing`, `identifying-patterns`, `guiding-doc-creation`, `running-benchmarks`, `auditing-skill-descriptions`.

- [ ] **Step 1: Write enable.md**

```markdown
---
description: Enable an opt-in skill for this project
argument-hint: Skill name (e.g. session-management)
---

Available opt-in skills:
- `session-management`
- `codebase-indexing`
- `identifying-patterns`
- `guiding-doc-creation`
- `running-benchmarks`
- `auditing-skill-descriptions`

If $ARGUMENTS is empty or invalid, list the above and ask which to enable.

Steps:
1. Derive project slug from CWD: replace `/` with `-`
   Example: `/Users/Jerry/Developer/myapp` → `-Users-Jerry-Developer-myapp`
2. Config path: `~/.claude/projects/<slug>/sensei.config.json`
3. Read existing config (or start with `{"skills":[]}`)
4. Add the skill if not already present
5. Write back
6. Confirm: "Enabled <skill> for this project. Takes effect next session."
```

- [ ] **Step 2: Write disable.md**

```markdown
---
description: Disable an opt-in skill for this project
argument-hint: Skill name
---

1. Derive project slug from CWD: replace `/` with `-`
2. Config path: `~/.claude/projects/<slug>/sensei.config.json`
3. Read existing config
4. Remove the skill if present
5. Write back
6. Confirm: "Disabled <skill> for this project. Takes effect next session."

If the config file doesn't exist or the skill isn't in the list, say so.
```

- [ ] **Step 3: Final install + verify full plugin**

```bash
bun run plugin:install
```

Expected output should now list all 10 skills and 12 commands.

- [ ] **Step 4: Commit**

```bash
git add plugin/commands/enable.md plugin/commands/disable.md
git commit -m "feat(plugin): add enable/disable commands for opt-in skill management"
```

---

## Chunk 5: Cleanup

### Task 16: Remove superseded skill files

Once the plugin is installed and verified, remove the original skill files to avoid confusion. The plugin is now the single source of truth.

- [ ] **Step 1: Verify plugin is working before deleting originals**

Start a fresh Claude Code session. Confirm:
- `working-smarter` appears in skills list
- `pattern-based-development` appears in skills list
- `/mockup`, `/commit`, `/session` commands are available

- [ ] **Step 2: Remove standalone global skills that are now in the plugin**

```bash
# Skills now packaged in the plugin — remove originals
rm ~/.claude/skills/working-smarter.md
rm -rf ~/.claude/skills/zero-errors-policy/
rm ~/.claude/skills/zero-errors-policy.md 2>/dev/null || true
rm ~/.claude/skills/context.md 2>/dev/null || true
rm ~/.claude/skills/workflow.md 2>/dev/null || true
rm ~/.claude/skills/orientation.md 2>/dev/null || true
rm ~/.claude/skills/patterns.md 2>/dev/null || true
rm ~/.claude/skills/identify-unknown-libs.md 2>/dev/null || true
```

- [ ] **Step 3: Remove project-local skills now covered by the plugin**

The skills in `sensei/skills/` that are now packaged in the plugin can be removed. Skills that have project-specific test files (test-baseline.md etc.) — preserve them until their contents are migrated.

```bash
# First: check which sensei/skills/ are fully covered
ls sensei/skills/
# Remove those that are now in plugin/skills/ with no additional test content
```

- [ ] **Step 4: Update .sensei/agent-skills.json**

Remove entries for skills that no longer exist at their original paths. The plugin-registered skills don't need entries here.

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "chore(plugin): remove superseded skill files now packaged in plugin"
```
