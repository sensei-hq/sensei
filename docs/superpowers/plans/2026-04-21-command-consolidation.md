# Command Consolidation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reduce 28 marketplace commands to 20 by dropping redundant commands, merging related ones, and improving documentation.

**Architecture:** Marketplace commands are markdown files in `marketplace/plugins/sensei/commands/`. The catalog (`marketplace/catalog.json`) indexes them. The fallback installer (`crates/senseid/src/installer.rs`) copies them to `~/.claude/commands/`. Hooks reference commands by name in their output. All changes are additive markdown edits except the installer cleanup logic (Rust).

**Tech Stack:** Markdown (commands, docs), Bash (hooks), Rust (installer), JSON (catalog)

---

### Task 1: Delete dropped command files and rename docs

**Files:**
- Delete: `marketplace/plugins/sensei/commands/enable.md`
- Delete: `marketplace/plugins/sensei/commands/disable.md`
- Delete: `marketplace/plugins/sensei/commands/pattern-extract.md`
- Delete: `marketplace/plugins/sensei/commands/pattern-use.md`
- Delete: `marketplace/plugins/sensei/commands/status.md`
- Delete: `marketplace/plugins/sensei/commands/refocus.md`
- Delete: `marketplace/plugins/sensei/commands/backlog.md`
- Delete: `marketplace/plugins/sensei/commands/product.md`
- Delete: `marketplace/plugins/sensei/commands/feature.md`
- Delete: `marketplace/plugins/sensei/commands/audit.md`
- Rename: `marketplace/plugins/sensei/commands/get-api-docs.md` → `marketplace/plugins/sensei/commands/docs.md`
- Modify: `marketplace/plugins/sensei/commands/docs.md` (update description)

- [ ] **Step 1: Delete 10 command files**

```bash
cd /Users/Jerry/Developer/sensei-dev
rm marketplace/plugins/sensei/commands/enable.md \
   marketplace/plugins/sensei/commands/disable.md \
   marketplace/plugins/sensei/commands/pattern-extract.md \
   marketplace/plugins/sensei/commands/pattern-use.md \
   marketplace/plugins/sensei/commands/status.md \
   marketplace/plugins/sensei/commands/refocus.md \
   marketplace/plugins/sensei/commands/backlog.md \
   marketplace/plugins/sensei/commands/product.md \
   marketplace/plugins/sensei/commands/feature.md \
   marketplace/plugins/sensei/commands/audit.md
```

- [ ] **Step 2: Rename get-api-docs.md → docs.md and update frontmatter**

```bash
mv marketplace/plugins/sensei/commands/get-api-docs.md marketplace/plugins/sensei/commands/docs.md
```

Then edit `marketplace/plugins/sensei/commands/docs.md` — change the frontmatter:

Old:
```yaml
---
description: Fetch documentation for a third-party library before writing code that uses it
argument-hint: <library-name> [component]
---
```

New:
```yaml
---
description: Fetch library documentation before writing code that uses it
argument-hint: <library-name> [component]
---
```

Also change the heading from `# Get API / Library Docs` to `# Library Docs`.

- [ ] **Step 3: Verify 18 command files remain**

```bash
ls marketplace/plugins/sensei/commands/ | wc -l
```

Expected: 18 (the 20 final commands minus `agent.md` and `spec.md` which are created in Tasks 2-3)

- [ ] **Step 4: Commit**

```bash
git add marketplace/plugins/sensei/commands/
git commit -m "refactor: drop 10 redundant commands, rename get-api-docs to docs

Removed: enable, disable, pattern-extract, pattern-use, status,
refocus, backlog, product, feature, audit.
Renamed: get-api-docs → docs.

Part of command consolidation (28→20)."
```

---

### Task 2: Create `agent.md` command

**Files:**
- Create: `marketplace/plugins/sensei/commands/agent.md`

- [ ] **Step 1: Create the command file**

Write `marketplace/plugins/sensei/commands/agent.md`:

```markdown
---
description: List available agents or invoke one by name
argument-hint: list | use <agent-name> [task description]
---

## What this command does

Manages sensei's mindset-based agents. List all available agents or dispatch one as a subagent for a specific task.

## Procedure

### Step 1: Parse action

Extract the first word of $ARGUMENTS:
- If empty or `list` → go to **List agents**
- If `use` → go to **Use agent**
- Otherwise → show usage: "Usage: `/sensei:agent list` or `/sensei:agent use <name> [task]`"

### List agents

Display all available agents:

| Agent | Description | When to use |
|-------|-------------|-------------|
| `analyst` | Problem analysis — requirements, constraints, scope | Before designing or building |
| `developer` | Implementation review — file placement, patterns, approach | Before coding |
| `acceptance-tester` | End-to-end testing — acceptance criteria, regressions | After implementation |
| `security-reviewer` | Security audit — OWASP, auth, data exposure, injection | User input, auth, data storage |
| `performance-engineer` | Performance analysis — complexity, memory, network, scalability | Data processing, queries, latency |
| `ux-designer` | UX review — usability, accessibility, consistency | Commands, UI, output formatting |
| `devops-sre` | Ops readiness — deployability, monitoring, rollback | Deployment, infra, reliability |
| `persona-reviewer` | Persona validation — review from persona perspective | Validate work against user goals |

Example: `/sensei:agent use security-reviewer check the auth middleware`

### Use agent

1. Extract agent name from $ARGUMENTS (second word after "use")
2. Extract task description (remaining words after agent name, if any)
3. Validate agent name matches one of the 8 agents above
4. If invalid name, list available agents and ask user to pick one
5. Dispatch the agent using the Agent tool:
   - Set `subagent_type` to `sensei:<agent-name>` (e.g., `sensei:sensei-security-reviewer`)
   - Pass the task description as the prompt
   - If no task provided, use: "Review the current work in progress"
6. Report the agent's findings to the user
```

- [ ] **Step 2: Verify file exists**

```bash
cat marketplace/plugins/sensei/commands/agent.md | head -5
```

Expected: frontmatter with description

- [ ] **Step 3: Commit**

```bash
git add marketplace/plugins/sensei/commands/agent.md
git commit -m "feat: add /sensei:agent command — list and invoke mindset agents"
```

---

### Task 3: Create `spec.md` command

**Files:**
- Create: `marketplace/plugins/sensei/commands/spec.md`

- [ ] **Step 1: Create the command file**

Write `marketplace/plugins/sensei/commands/spec.md`:

```markdown
---
description: Reverse-engineer docs — product overview, feature deep-dive, or security audit
argument-hint: product [path] | feature <name> | audit [name]
---

## What this command does

Generates structured documentation by reverse-engineering the codebase. Three modes available — product overview, feature deep-dive, and security/quality audit. All delegate to the `sensei:reverse-engineering` skill.

## Procedure

### Step 1: Parse mode

Extract the first word of $ARGUMENTS:
- `product` → Product mode
- `feature` → Feature mode
- `audit` → Audit mode
- Empty or anything else → show usage:

```
Usage:
  /sensei:spec product [root-path]    — Reverse-engineer the full product
  /sensei:spec feature <name>         — Deep-dive a specific feature
  /sensei:spec audit [capability]     — OWASP, NFR, and quality audit

Examples:
  /sensei:spec product
  /sensei:spec feature auth
  /sensei:spec audit payments
```

### Product mode

Use the Skill tool to invoke `sensei:reverse-engineering`, then execute it with:

```
mode=product root=<remaining args or ".">
```

Generates `openspec/product/` docs. If no root path given, defaults to `.`.

### Feature mode

Use the Skill tool to invoke `sensei:reverse-engineering`, then execute it with:

```
mode=feature capability=<remaining args>
```

If no capability name given, run interactive feature selection (requires `openspec/product/features.md` — run `/sensei:spec product` first if missing).

Generates `openspec/specs/<capability>/` docs.

### Audit mode

Use the Skill tool to invoke `sensei:reverse-engineering`, then execute it with:

```
mode=audit capability=<remaining args>
```

If no capability specified, audit ALL capabilities under `openspec/specs/` plus `specs/common/` and all DB docs.

Covers:
- OWASP 2021 security checks
- NFR coverage across 6 dimensions
- Code quality health score
- Drift detection against current source
```

- [ ] **Step 2: Verify file exists**

```bash
cat marketplace/plugins/sensei/commands/spec.md | head -5
```

Expected: frontmatter with description

- [ ] **Step 3: Commit**

```bash
git add marketplace/plugins/sensei/commands/spec.md
git commit -m "feat: add /sensei:spec command — consolidates product, feature, audit"
```

---

### Task 4: Rewrite `session.md` to absorb status/refocus/backlog

**Files:**
- Modify: `marketplace/plugins/sensei/commands/session.md`

- [ ] **Step 1: Rewrite session.md**

Replace full contents of `marketplace/plugins/sensei/commands/session.md` with:

```markdown
---
description: Session management — resume, show state, re-anchor, or list open work
argument-hint: (none) | status | refocus | backlog
---

## What this command does

Central command for session and workflow state. Four sub-actions:

| Sub-action | What it does |
|------------|-------------|
| *(no args)* | Resume session — load context, surface open decisions |
| `status` | Full orientation — phase, task, issue, rules, patterns, docs |
| `refocus` | Re-anchor on current task — reload state, plan, rules |
| `backlog` | List open tasks, decisions, pending questions |

## Procedure

### Step 1: Parse action

Extract the first word of $ARGUMENTS:
- Empty → **Resume**
- `status` → **Status**
- `refocus` → **Refocus**
- `backlog` → **Backlog**
- Anything else → treat as task description for Resume

### Resume (default — no args)

1. Call `get_session_context(task_description="session startup")`
2. Review any open decisions or interrupted work it returns
3. Report back: what's in progress, what's pending, any blockers

### Status

Display full orientation — "where am I" across all dimensions. Read-only.

1. **Workflow state**: Call `get_workflow_state()`. Display active_phase, active_task, active_issue, last_checkpoint. If empty, say "No active workflow state."

2. **Rules**: Check if `.sensei/rules.md` exists. Count rules (lines starting with `- **`). Display: "Rules: .sensei/rules.md (N rules loaded)" or "Rules: not configured"

3. **Patterns**: Call `get_patterns(pattern="")` to get all detected patterns. Display count.

4. **Docs**: Count files in each phase folder:
   - docs/ideas/, docs/analysis/, docs/blueprints/, docs/experiments/, docs/plans/
   Display counts. Skip folders that don't exist.

5. **Open issues** (if gh CLI available): Run `gh issue list --state open --limit 5 --json number,title` and display top 5.

Output format:
```
Phase:      [phase or "none"]
Task:       [task or "none"]
Issue:      #[number] or "none"
Checkpoint: [timestamp or "none"]

Rules:      .sensei/rules.md ([N] rules)
Patterns:   [N] detected patterns

Docs:
  ideas/       [N] files
  analysis/    [N] files
  blueprints/  [N] files

Open issues: [top 5 or "no gh CLI"]
```

### Refocus

Re-anchor on the current task after drift or context compaction. Read-only.

1. Read `.sensei/state.yaml` — extract active_phase, active_task, active_issue. If missing: "No active workflow state — use a phase command to set one."
2. If state has active_plan, read that file. Extract current feature and acceptance criteria.
3. If state has active_issue and gh CLI is available, run `gh issue view [number] --json title,body,labels`.
4. Read `.sensei/rules.md` — output compact summary (section headers + rule count).
5. Output orientation:
   - "Current phase: [phase]"
   - "Active task: [task]"
   - "Issue: #[number] — [title]"
   - "Acceptance criteria: [from plan or issue]"
   - "Rules: [count] rules loaded"
   - "What's left: [remaining items from plan]"
6. If conversation shows work unrelated to the active task, acknowledge: "I had drifted to [topic]. Returning to: [active task]."

### Backlog

1. Call `get_session_context(task_description="backlog review")`
2. Extract and display:
   - **Open decisions** — things awaiting a choice
   - **Pending tasks** — work in progress or queued
   - **Blocked items** — waiting on external input
   - **Questions** — unresolved from previous sessions
3. Format as a prioritized list. If session store is empty, say so.
```

- [ ] **Step 2: Verify**

```bash
grep "argument-hint" marketplace/plugins/sensei/commands/session.md
```

Expected: `argument-hint: (none) | status | refocus | backlog`

- [ ] **Step 3: Commit**

```bash
git add marketplace/plugins/sensei/commands/session.md
git commit -m "feat: session command absorbs status, refocus, backlog sub-actions"
```

---

### Task 5: Rewrite `help.md` with examples and agent reference

**Files:**
- Modify: `marketplace/plugins/sensei/commands/help.md`

- [ ] **Step 1: Rewrite help.md**

Replace full contents of `marketplace/plugins/sensei/commands/help.md` with:

```markdown
---
description: Show all available sensei commands, agents, and usage examples
---

# Sensei — Command Reference

## Quick Start

```
/sensei:session              # Resume where you left off
/sensei:idea <topic>         # Start exploring a concept
/sensei:build #42            # Implement an issue
/sensei:help                 # This reference
```

---

## Phase Commands — The workflow pipeline

| Command | Description | Example |
|---------|-------------|---------|
| `/sensei:idea <topic>` | Explore a concept — questions, problem space | `/sensei:idea task scheduler` |
| `/sensei:analyze [topic]` | Feasibility — 2-3 options with tradeoffs | `/sensei:analyze` |
| `/sensei:blueprint [topic]` | Architecture — components, interfaces, data flow | `/sensei:blueprint caching layer` |
| `/sensei:experiment <topic>` | Prototype — build minimal, document findings | `/sensei:experiment RxJS for real-time` |
| `/sensei:plan [topic]` | Decompose into features with acceptance criteria | `/sensei:plan` |
| `/sensei:build <#issue\|desc>` | Implement — locate code, TDD, review | `/sensei:build #42` |
| `/sensei:validate [#issue]` | Verify — tests, acceptance criteria, doc drift | `/sensei:validate` |

Typical flow: `idea` → `analyze` → `blueprint` → `plan` → `build` → `validate`

## Cross-cutting Commands

| Command | Description | Example |
|---------|-------------|---------|
| `/sensei:brainstorm [topic]` | Open creative exploration | `/sensei:brainstorm` |
| `/sensei:review [scope]` | Quality check — patterns, tests, coverage | `/sensei:review modified files` |
| `/sensei:persona <action>` | Manage project personas | `/sensei:persona add end-user` |
| `/sensei:agent <action>` | List or invoke mindset agents | `/sensei:agent use security-reviewer` |

### Persona sub-actions

| Action | Example |
|--------|---------|
| `list` | `/sensei:persona list` |
| `add <name>` | `/sensei:persona add mobile-user` |
| `switch <name>` | `/sensei:persona switch admin` |
| `validate` | `/sensei:persona validate` |

### Agent sub-actions

| Action | Example |
|--------|---------|
| `list` | `/sensei:agent list` |
| `use <name> [task]` | `/sensei:agent use analyst review the auth design` |

## Utility Commands

| Command | Description | Example |
|---------|-------------|---------|
| `/sensei:session [action]` | Session & state management | `/sensei:session status` |
| `/sensei:spec <mode>` | Reverse-engineer docs | `/sensei:spec feature auth` |
| `/sensei:rules [rule]` | View or add project rules | `/sensei:rules` |
| `/sensei:patterns [query]` | Show detected patterns | `/sensei:patterns adapter` |
| `/sensei:checkpoint [summary]` | Snapshot progress | `/sensei:checkpoint auth done` |
| `/sensei:commit [message]` | Zero-errors check + commit | `/sensei:commit` |
| `/sensei:mockup <desc>` | Start a UI mockup | `/sensei:mockup dashboard` |
| `/sensei:docs <lib>` | Fetch library docs | `/sensei:docs sveltekit hooks` |

### Session sub-actions

| Action | Example |
|--------|---------|
| *(no args)* — resume | `/sensei:session` |
| `status` — full orientation | `/sensei:session status` |
| `refocus` — re-anchor on task | `/sensei:session refocus` |
| `backlog` — open work items | `/sensei:session backlog` |

### Spec sub-actions

| Action | Example |
|--------|---------|
| `product [path]` | `/sensei:spec product` |
| `feature <name>` | `/sensei:spec feature payments` |
| `audit [name]` | `/sensei:spec audit` |

---

## Agents

Mindset-based subagents — invoke with `/sensei:agent use <name>` or they activate automatically based on context.

| Agent | Focus | When to use |
|-------|-------|-------------|
| `analyst` | Requirements, constraints, scope | Before designing |
| `developer` | Implementation approach, patterns | Before coding |
| `acceptance-tester` | End-to-end testing, regressions | After implementation |
| `security-reviewer` | OWASP, auth, data exposure | User input, auth, data storage |
| `performance-engineer` | Complexity, memory, scalability | Queries, loops, latency |
| `ux-designer` | Usability, accessibility | UI, commands, output |
| `devops-sre` | Deploy, monitoring, rollback | Infra, config, reliability |
| `persona-reviewer` | Persona perspective validation | Validate against user goals |

## Skills (auto-applied)

| Skill | Triggers when... |
|-------|-----------------|
| `codebase-indexing` | First working on a repo or after major refactor |
| `analyze` | Deep architecture analysis needed |
| `reverse-engineering` | Reverse-engineering an unfamiliar codebase |
| `test-gen` | Adding test coverage to untested code |
| `refactor` | Improving code structure without changing behaviour |
| `extract-docs` | Generating docs from code |
| `building-app-mockups` | Building interactive mockups |
| `identify-unknown-libs` | Library docs missing from index |
```

- [ ] **Step 2: Verify**

```bash
grep -c "sensei:" marketplace/plugins/sensei/commands/help.md
```

Expected: 30+ references (all commands listed)

- [ ] **Step 3: Commit**

```bash
git add marketplace/plugins/sensei/commands/help.md
git commit -m "docs: rewrite help command with examples and agent reference"
```

---

### Task 6: Update catalog.json

**Files:**
- Modify: `marketplace/catalog.json`

- [ ] **Step 1: Remove 10 dropped command entries**

Remove JSON entries with names: `enable`, `disable`, `pattern-extract`, `pattern-use`, `status`, `refocus`, `backlog`, `product`, `feature`, `audit`.

- [ ] **Step 2: Rename get-api-docs entry to docs**

Change:
```json
{
  "name": "get-api-docs",
  ...
  "path": "commands/get-api-docs.md"
}
```
To:
```json
{
  "name": "docs",
  "kind": "command",
  "description": "Fetch library documentation before writing code that uses it",
  "scope": "global",
  "recommended_for": ["all"],
  "stage": ["active"],
  "path": "commands/docs.md"
}
```

- [ ] **Step 3: Update session entry description**

Change session description to:
```json
"description": "Session management — resume, show state, re-anchor, list open work"
```

- [ ] **Step 4: Add agent entry**

Add after the `persona` entry:
```json
{
  "name": "agent",
  "kind": "command",
  "description": "List available agents or invoke one by name",
  "scope": "project",
  "recommended_for": ["all"],
  "stage": ["active"],
  "path": "commands/agent.md"
}
```

- [ ] **Step 5: Add spec entry**

Add after the `rules` entry:
```json
{
  "name": "spec",
  "kind": "command",
  "description": "Reverse-engineer docs — product overview, feature deep-dive, or security audit",
  "scope": "project",
  "recommended_for": ["all"],
  "stage": ["active"],
  "path": "commands/spec.md"
}
```

- [ ] **Step 6: Verify catalog item count**

```bash
grep '"kind": "command"' marketplace/catalog.json | wc -l
```

Expected: 20 (was 28)

- [ ] **Step 7: Verify JSON is valid**

```bash
python3 -c "import json; json.load(open('marketplace/catalog.json'))" && echo "OK"
```

Expected: `OK`

- [ ] **Step 8: Commit**

```bash
git add marketplace/catalog.json
git commit -m "refactor: update catalog.json — 28 commands reduced to 20"
```

---

### Task 7: Add stale file cleanup to installer

**Files:**
- Modify: `crates/senseid/src/installer.rs:126-175`
- Modify: `crates/senseid/src/installer.rs` (InstallResult struct)

- [ ] **Step 1: Add stale count fields to InstallResult**

In `crates/senseid/src/installer.rs`, find:

```rust
pub struct InstallResult {
    pub hooks_installed: u32,
    pub skills_installed: u32,
    pub commands_installed: u32,
    pub acps_configured: Vec<String>,
    pub errors: Vec<String>,
    pub marketplace_version: String,
}
```

Add two fields:

```rust
pub struct InstallResult {
    pub hooks_installed: u32,
    pub skills_installed: u32,
    pub commands_installed: u32,
    pub stale_commands_removed: u32,
    pub stale_skills_removed: u32,
    pub acps_configured: Vec<String>,
    pub errors: Vec<String>,
    pub marketplace_version: String,
}
```

- [ ] **Step 2: Add cleanup function**

After the `install_marketplace` function (around line 175), add:

```rust
/// Remove command/skill files that are no longer in the catalog.
fn cleanup_stale_items(catalog: &Catalog) -> (u32, u32) {
    let h = home();
    let command_names: std::collections::HashSet<String> = catalog
        .items
        .iter()
        .filter(|i| i.kind == "command")
        .map(|i| format!("{}.md", i.name))
        .collect();
    let skill_names: std::collections::HashSet<String> = catalog
        .items
        .iter()
        .filter(|i| i.kind == "skill")
        .map(|i| format!("{}.md", i.name))
        .collect();

    let commands_removed = remove_stale_in(&h.join(".claude/commands"), &command_names);
    let skills_removed = remove_stale_in(&h.join(".claude/skills"), &skill_names);
    (commands_removed, skills_removed)
}

/// Remove .md files in `dir` whose names are not in `keep`.
fn remove_stale_in(dir: &std::path::Path, keep: &std::collections::HashSet<String>) -> u32 {
    let mut removed = 0u32;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".md") && !keep.contains(&name) {
                if fs::remove_file(entry.path()).is_ok() {
                    removed += 1;
                }
            }
        }
    }
    removed
}
```

- [ ] **Step 3: Call cleanup in install_marketplace**

In `install_marketplace`, before the `Ok(...)` return at line ~174, add the cleanup call:

Find:
```rust
    // Save version to ~/.sensei/config.json
    save_marketplace_version(&version);

    Ok((skills, commands, version))
```

Replace with:
```rust
    // Clean up stale items from previous versions
    let (stale_cmds, stale_skills) = cleanup_stale_items(&catalog);

    // Save version to ~/.sensei/config.json
    save_marketplace_version(&version);

    Ok((skills, commands, stale_cmds, stale_skills, version))
```

- [ ] **Step 4: Update install_marketplace return type and caller**

Change `install_marketplace` signature to return `(u32, u32, u32, u32, String)` — (skills, commands, stale_cmds, stale_skills, version).

Update the caller in `install()`:
```rust
    match install_marketplace(scope, acps) {
        Ok((skills, commands, stale_cmds, stale_skills, version)) => {
            result.skills_installed = skills;
            result.commands_installed = commands;
            result.stale_commands_removed = stale_cmds;
            result.stale_skills_removed = stale_skills;
            result.marketplace_version = version;
        }
        Err(e) => result.errors.push(format!("marketplace: {}", e)),
    }
```

- [ ] **Step 5: Run tests**

```bash
cargo test --manifest-path crates/senseid/Cargo.toml 2>&1 | grep "test result:"
```

Expected: all pass, 0 failed

- [ ] **Step 6: Run clippy**

```bash
cargo clippy --manifest-path crates/senseid/Cargo.toml 2>&1 | tail -3
```

Expected: 0 warnings

- [ ] **Step 7: Commit**

```bash
git add crates/senseid/src/installer.rs
git commit -m "feat: clean up stale command/skill files on marketplace install"
```

---

### Task 8: Update hooks

**Files:**
- Modify: `marketplace/plugins/sensei/hooks/session-start` (lines 148-153)
- Modify: `marketplace/plugins/sensei/hooks/pre-compact` (line 55)

- [ ] **Step 1: Update session-start hook**

In `marketplace/plugins/sensei/hooks/session-start`, find the `## Workflow Commands` block (around line 148-153):

```
## Workflow Commands

**Phase:** /sensei:idea, /sensei:analyze, /sensei:blueprint, /sensei:experiment, /sensei:plan, /sensei:build, /sensei:validate
**Cross-cutting:** /sensei:brainstorm, /sensei:review, /sensei:persona
**Refocus:** /sensei:rules, /sensei:refocus, /sensei:status, /sensei:tools, /sensei:patterns
**Utility:** /sensei:session, /sensei:checkpoint, /sensei:commit, /sensei:mockup, /sensei:pattern-extract, /sensei:get-api-docs, /sensei:help
```

Replace with:

```
## Workflow Commands

**Phase:** /sensei:idea, /sensei:analyze, /sensei:blueprint, /sensei:experiment, /sensei:plan, /sensei:build, /sensei:validate
**Cross-cutting:** /sensei:brainstorm, /sensei:review, /sensei:persona, /sensei:agent
**Context:** /sensei:session, /sensei:rules, /sensei:patterns
**Utility:** /sensei:spec, /sensei:checkpoint, /sensei:commit, /sensei:mockup, /sensei:docs, /sensei:help
```

- [ ] **Step 2: Update session-start state section**

Find (around line 104):
```
Run /sensei:status for full orientation. Run /sensei:refocus to re-anchor."
```

Replace with:
```
Run /sensei:session status for full orientation. Run /sensei:session refocus to re-anchor."
```

- [ ] **Step 3: Update pre-compact hook**

In `marketplace/plugins/sensei/hooks/pre-compact`, find (line 55):
```
Run /sensei:refocus for full orientation after compaction.
```

Replace with:
```
Run /sensei:session refocus for full orientation after compaction.
```

- [ ] **Step 4: Commit**

```bash
git add marketplace/plugins/sensei/hooks/session-start marketplace/plugins/sensei/hooks/pre-compact
git commit -m "docs: update hook command references for consolidated commands"
```

---

### Task 9: Rewrite marketplace/README.md

**Files:**
- Modify: `marketplace/README.md`

- [ ] **Step 1: Read current README**

```bash
cat marketplace/README.md
```

- [ ] **Step 2: Rewrite as comprehensive reference**

Replace full contents of `marketplace/README.md` with a comprehensive reference document covering:

1. **Overview** — What the sensei marketplace plugin provides
2. **Commands** (20) — grouped by category with description, arguments, examples
3. **Agents** (8) — name, description, model, when to use
4. **Skills** (9) — name, description, trigger conditions
5. **Hooks** — session-start, user-prompt, pre-compact, pre-tool, post-tool
6. **Workflow walkthrough** — typical idea → validate pipeline
7. **Installation** — `sensei init`

The content should mirror `help.md` but with more detail — full descriptions, multiple examples per command, and explanation of sub-actions.

- [ ] **Step 3: Commit**

```bash
git add marketplace/README.md
git commit -m "docs: comprehensive marketplace README with commands, agents, skills reference"
```

---

### Task 10: Update design and ideas docs

**Files:**
- Modify: 16 documentation files (see spec for full list)

All changes are search-and-replace of old command names to new names. These are historical docs so we update references inline.

- [ ] **Step 1: Batch search-and-replace across all docs**

Apply these replacements across all files in `docs/`:

| Old | New |
|-----|-----|
| `/sensei:refocus` | `/sensei:session refocus` |
| `/sensei:status` | `/sensei:session status` |
| `/sensei:backlog` | `/sensei:session backlog` |
| `/sensei:pattern-extract` | *(remove or note "removed — MCP auto-detects")* |
| `/sensei:pattern-use` | `/sensei:patterns` |
| `/sensei:product` | `/sensei:spec product` |
| `/sensei:feature` | `/sensei:spec feature` |
| `/sensei:audit` | `/sensei:spec audit` |
| `/sensei:get-api-docs` | `/sensei:docs` |
| `/sensei:enable` | *(remove or note "removed")* |
| `/sensei:disable` | *(remove or note "removed")* |

Files to update:
- `docs/README.md`
- `docs/features/02-rules-context.md`
- `docs/analysis/01-skill-command-mapping.md`
- `docs/ideas/01-workflow-system.md`
- `docs/ideas/02-commands.md`
- `docs/ideas/04-cross-cutting.md`
- `docs/ideas/05-decisions.md`
- `docs/ideas/06-docs-disposition.md`
- `docs/ideas/14-context-delivery.md`
- `docs/ideas/15-pattern-store.md`
- `docs/ideas/17-pattern-knowledge.md`
- `docs/blueprints/01-workflow-engine.md`
- `docs/design/02-mcp/workflow-tools.md`
- `docs/design/03-marketplace/plugin-packaging.md`
- `docs/design/03-marketplace/commands.md`
- `docs/design/roadmap.md`

- [ ] **Step 2: Special handling for docs/ideas/02-commands.md**

This is the command design doc. Add a note at the top:

```markdown
> **Updated 2026-04-21:** Commands consolidated from 28 to 20.
> See `docs/superpowers/specs/2026-04-21-command-consolidation-design.md` for the consolidation spec.
```

Update the command tables to reflect the final 20 commands.

- [ ] **Step 3: Verify no stale references remain**

```bash
grep -rn "sensei:refocus\|sensei:status\|sensei:backlog\|sensei:pattern-extract\|sensei:pattern-use\|sensei:product\b\|sensei:feature\b\|sensei:audit\b\|sensei:get-api-docs\|sensei:enable\|sensei:disable" docs/ | grep -v "session refocus\|session status\|session backlog\|spec product\|spec feature\|spec audit\|sensei:docs\|removed\|consolidated\|superseded"
```

Expected: 0 matches (all references updated or annotated)

- [ ] **Step 4: Commit**

```bash
git add docs/
git commit -m "docs: update all design/ideas docs for command consolidation (28→20)"
```

---

### Task 11: Final verification

**Files:** None (verification only)

- [ ] **Step 1: Verify command file count**

```bash
ls marketplace/plugins/sensei/commands/ | wc -l
```

Expected: 20

- [ ] **Step 2: Verify catalog command count**

```bash
grep '"kind": "command"' marketplace/catalog.json | wc -l
```

Expected: 20

- [ ] **Step 3: Run Rust tests**

```bash
cargo test --manifest-path crates/senseid/Cargo.toml 2>&1 | grep "test result:"
```

Expected: all pass, 0 failed

- [ ] **Step 4: Run clippy**

```bash
cargo clippy --manifest-path crates/senseid/Cargo.toml 2>&1 | tail -3
cargo clippy --manifest-path crates/sensei-cli/Cargo.toml 2>&1 | tail -3
```

Expected: 0 warnings for both

- [ ] **Step 5: Verify no broken references in hooks**

```bash
grep -n "sensei:" marketplace/plugins/sensei/hooks/session-start | head -20
grep -n "sensei:" marketplace/plugins/sensei/hooks/pre-compact | head -10
```

Verify all referenced commands exist as files in `marketplace/plugins/sensei/commands/`.

- [ ] **Step 6: Final commit (if any fixups needed)**

```bash
git status
```

If clean: done. If not: fix, commit.
