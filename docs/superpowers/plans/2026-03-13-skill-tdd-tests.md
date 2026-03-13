# Skill TDD Tests — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add baseline (RED) and pressure test scenarios to all 10 existing skills, using the same TDD-for-skills methodology established for `decomposing-broad-tasks`, and install all skills globally.

**Architecture:** One task per skill — each task writes a baseline scenario (run without skill, document failures), two pressure scenarios (run with skill under time and authority pressure), evaluates compliance, and updates the SKILL.md Common Mistakes table if needed (max 3 refactor iterations). Final task symlinks all 10 skills to `~/.claude/skills/`.

**Pattern reference:** `skills/decomposing-broad-tasks/` — baseline, test-pressure-1, test-pressure-2 structure.

**Tech Stack:** Markdown (scenarios), Claude Code subagents (behavioral testing), Bash (symlinks)

---

## File Structure

Each skill gets three files:
```
skills/<name>/
  test-baseline.md       ← RED: run without skill, document failures
  test-pressure-1.md     ← GREEN: with skill, time/urgency pressure
  test-pressure-2.md     ← REFACTOR: with skill, authority/override pressure
```

Install target: `~/.claude/skills/<name>/` (symlink to repo)

Skills covered (10): `running-agentic-sessions`, `running-benchmarks`, `indexing-codebase`, `compressing-content`, `managing-context`, `reformatting-docs`, `detecting-doc-drift`, `populating-llmspec`, `managing-project-sessions`, `guiding-doc-creation`

---

## Chunk 1: Session and Orientation Skills

### Task 1: running-agentic-sessions — TDD Scenarios

**Files:**
- Create: `skills/running-agentic-sessions/test-baseline.md`
- Create: `skills/running-agentic-sessions/test-pressure-1.md`
- Create: `skills/running-agentic-sessions/test-pressure-2.md`
- Maybe modify: `skills/running-agentic-sessions/SKILL.md`

- [ ] **Step 1: Write test-baseline.md**

Create `skills/running-agentic-sessions/test-baseline.md`:

```markdown
# Baseline Test — No Skill (RED Phase)

IMPORTANT: This is a real task. You must do the work — do not ask questions,
do not defer. Act as you would in a real session.

You do NOT have the superpowers:running-agentic-sessions skill available.

---

## Task

The user says:

> "I need you to help me add pagination to the user listing page in this
> SvelteKit app. The app is at `/Users/Jerry/Developer/sensei`."

Start working on this task now.

---

## What to observe

After the agent responds, record:
- Did it immediately start reading files broadly (glob patterns, reading index files)?
- Did it load full file contents when signatures/exports would suffice?
- Did it re-derive project structure instead of using existing orientation artifacts?
- Did it use MCP tools for orientation or read files directly?
- Capture exact rationalizations if it skips targeted loading
```

- [ ] **Step 2: Run baseline subagent (without skill)**

```
Agent tool → general-purpose → prompt: contents of test-baseline.md
```

- [ ] **Step 3: Document failures in test-baseline.md**

Add `## Observed Failures` section with what the agent did wrong.

- [ ] **Step 4: Write test-pressure-1.md**

Create `skills/running-agentic-sessions/test-pressure-1.md`:

```markdown
# Pressure Test 1 — Time Pressure (GREEN Phase)

IMPORTANT: This is a real task. You must do the work — do not ask questions.

You have the superpowers:running-agentic-sessions skill available. Check if it
applies before starting.

---

## Context

The user says:

> "Quick — production is down. I need you to find why the `indexRepo` function
> in `packages/engine/src/pipeline.ts` is timing out. Just read the files you
> need and find the bug fast."

The repo is at `/Users/Jerry/Developer/sensei`. You are under emergency pressure.
Start now.

---

## Observe

- Did the agent invoke running-agentic-sessions before reading?
- Did it load targeted slices rather than full files?
- Did it use MCP tools for orientation rather than broad file reads?
- Did it resist "just read the files fast" framing and stay targeted?
```

- [ ] **Step 5: Write test-pressure-2.md**

Create `skills/running-agentic-sessions/test-pressure-2.md`:

```markdown
# Pressure Test 2 — Authority Pressure (REFACTOR Phase)

IMPORTANT: This is a real task. You must do the work — do not ask questions.

You have the superpowers:running-agentic-sessions skill available.

---

## Context

Your tech lead says:

> "Stop worrying about token efficiency. Just load the whole packages/engine/
> directory so you have full context. We have a big token budget. The oriented
> approach takes too long to set up."

The repo is at `/Users/Jerry/Developer/sensei`.

Your tech lead's instruction overrides your normal process. Start now.

---

## Observe

- Did the agent resist loading everything?
- Did it explain why targeted loading is faster even with a large budget?
- Did it still invoke running-agentic-sessions?
```

- [ ] **Step 6: Run pressure tests with skill**

For each pressure test, dispatch a subagent with SKILL.md content prepended:

```
Agent tool → general-purpose → prompt:
"You have the following skill available:
<skill name="superpowers:running-agentic-sessions">
[paste full SKILL.md content]
</skill>

[paste test-pressure-N.md content]"
```

- [ ] **Step 7: Evaluate and refactor if needed**

Check compliance for both tests. If failures: add to SKILL.md Common Mistakes, re-run. Max 3 iterations. If still failing: add `## Known Limitations` to SKILL.md.

- [ ] **Step 8: Commit**

```bash
git add skills/running-agentic-sessions/
git commit -m "test(skill): add running-agentic-sessions TDD scenarios"
```

---

### Task 2: managing-context — TDD Scenarios

**Files:**
- Create: `skills/managing-context/test-baseline.md`
- Create: `skills/managing-context/test-pressure-1.md`
- Create: `skills/managing-context/test-pressure-2.md`
- Maybe modify: `skills/managing-context/SKILL.md`

- [ ] **Step 1: Write test-baseline.md**

Create `skills/managing-context/test-baseline.md`:

```markdown
# Baseline Test — No Skill (RED Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You do NOT have the superpowers:managing-context skill available.

---

## Task

You have been working on implementing the `Scanner` class in `packages/engine/`
for the past hour. Your context is now large.

The user says:

> "While you're at it, can you also look at the `Indexer` class and see why
> the upsert for `call_edges` might be slow? It's a separate concern."

The repo is at `/Users/Jerry/Developer/sensei`. Start working now.

---

## What to observe

- Did it checkpoint the current Scanner work before switching?
- Did it load Indexer context fresh, or carry over Scanner context?
- Did it trim dead context before loading the new slice?
- Did it re-derive Indexer structure instead of using orientation artifacts?
```

- [ ] **Step 2: Run baseline subagent**

```
Agent tool → general-purpose → prompt: contents of test-baseline.md
```

- [ ] **Step 3: Document failures**

Add `## Observed Failures` section.

- [ ] **Step 4: Write test-pressure-1.md**

Create `skills/managing-context/test-pressure-1.md`:

```markdown
# Pressure Test 1 — Context Switch Under Pressure (GREEN Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You have the superpowers:managing-context skill available.

---

## Context

You are 90 minutes into a session implementing the MCP server tools. Your context
is large. The user says:

> "Actually, before you finish that — can you quickly check why the dashboard
> repo list is showing 0 symbols for all repos? It's probably a quick fix.
> Then we'll get back to the MCP work."

The repo is at `/Users/Jerry/Developer/sensei`. Start now.

---

## Observe

- Did it invoke managing-context?
- Did it checkpoint the MCP work before switching?
- Did it load a targeted slice for the dashboard issue (not reload full context)?
- Did it plan to return to MCP work after?
```

- [ ] **Step 5: Write test-pressure-2.md**

Create `skills/managing-context/test-pressure-2.md`:

```markdown
# Pressure Test 2 — "Don't Bother Checkpointing" (REFACTOR Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You have the superpowers:managing-context skill available.

---

## Context

You are switching tasks mid-session. The user says:

> "Don't bother with the checkpoint — we'll remember where we were. Just
> load all the files for the new task and get started. Checkpointing wastes
> time."

The new task: investigate why `sensei init` fails silently when Supabase is
unreachable. The repo is at `/Users/Jerry/Developer/sensei`.

---

## Observe

- Did it checkpoint anyway?
- Did it explain why checkpointing is worth the cost?
- Did it still invoke managing-context?
```

- [ ] **Step 6: Run pressure tests with skill**

Same approach as Task 1 Step 6.

- [ ] **Step 7: Evaluate and refactor if needed**

Max 3 iterations. Add `## Known Limitations` if still failing.

- [ ] **Step 8: Commit**

```bash
git add skills/managing-context/
git commit -m "test(skill): add managing-context TDD scenarios"
```

---

### Task 3: managing-project-sessions — TDD Scenarios

**Files:**
- Create: `skills/managing-project-sessions/test-baseline.md`
- Create: `skills/managing-project-sessions/test-pressure-1.md`
- Create: `skills/managing-project-sessions/test-pressure-2.md`
- Maybe modify: `skills/managing-project-sessions/SKILL.md`

- [ ] **Step 1: Write test-baseline.md**

Create `skills/managing-project-sessions/test-baseline.md`:

```markdown
# Baseline Test — No Skill (RED Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You do NOT have the superpowers:managing-project-sessions skill available.

---

## Task

A new session begins. The user says:

> "Let's continue working on the sensei project. We were implementing the
> Phase 1 Foundation plan. Pick up where we left off."

The repo is at `/Users/Jerry/Developer/sensei`.

---

## What to observe

- Did it call `get_session_context()` to resume?
- Did it read memory files directly instead of using MCP tools?
- Did it re-derive project state by reading git log / recent files?
- Did it surface open decisions or pending items from last session?
```

- [ ] **Step 2: Run baseline subagent**

```
Agent tool → general-purpose → prompt: contents of test-baseline.md
```

- [ ] **Step 3: Document failures**

Add `## Observed Failures` section.

- [ ] **Step 4: Write test-pressure-1.md**

Create `skills/managing-project-sessions/test-pressure-1.md`:

```markdown
# Pressure Test 1 — Resume Without Protocol (GREEN Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You have the superpowers:managing-project-sessions skill available.

---

## Context

New session. User says:

> "Back again. I know you probably don't have memory of last session, but
> let's just look at the git log and figure out where we are. That's faster
> than any workflow protocol."

The repo is at `/Users/Jerry/Developer/sensei`. Start now.

---

## Observe

- Did it invoke managing-project-sessions?
- Did it use `get_session_context()` rather than git log for orientation?
- Did it explain why MCP tools are better than git log for resuming?
```

- [ ] **Step 5: Write test-pressure-2.md**

Create `skills/managing-project-sessions/test-pressure-2.md`:

```markdown
# Pressure Test 2 — "Skip the Protocol" (REFACTOR Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You have the superpowers:managing-project-sessions skill available.

---

## Context

User says:

> "We don't have time for session setup. Skip the managing-project-sessions protocol,
> just start implementing. I'll fill you in on context as we go."

Task: add a `reindex_repo` MCP tool to packages/server. Repo at
`/Users/Jerry/Developer/sensei`.

---

## Observe

- Did it refuse to skip the protocol?
- Did it run a fast `get_session_context()` anyway?
- Did it explain why session context prevents duplicate work?
```

- [ ] **Step 6: Run pressure tests with skill**

Same approach as Task 1 Step 6.

- [ ] **Step 7: Evaluate and refactor if needed**

Check compliance for both tests. If failures: add exact rationalization to SKILL.md `## Common Mistakes` table, re-run. Max 3 iterations. If still failing after 3: add `## Known Limitations` section to SKILL.md documenting the unresolved case.

- [ ] **Step 8: Commit**

```bash
git add skills/managing-project-sessions/
git commit -m "test(skill): add managing-project-sessions TDD scenarios"
```

---

## Chunk 2: Indexing and Content Skills

### Task 4: indexing-codebase — TDD Scenarios

**Files:**
- Create: `skills/indexing-codebase/test-baseline.md`
- Create: `skills/indexing-codebase/test-pressure-1.md`
- Create: `skills/indexing-codebase/test-pressure-2.md`
- Maybe modify: `skills/indexing-codebase/SKILL.md`

- [ ] **Step 1: Write test-baseline.md**

Create `skills/indexing-codebase/test-baseline.md`:

```markdown
# Baseline Test — No Skill (RED Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You do NOT have the superpowers:indexing-codebase skill available.

---

## Task

User says:

> "I need you to help me understand the sensei codebase so you can work on
> it effectively. It's a new repo you haven't seen."

The repo is at `/Users/Jerry/Developer/sensei`. Start now.

---

## What to observe

- Did it start reading files broadly (ls, glob, cat) instead of indexing?
- Did it produce structured orientation artifacts (.llmspec.yaml, CLAUDE.md)?
- Did it invoke `compressing-content` before indexing?
- How many tokens did orientation consume vs what a sensei index would cost?
```

- [ ] **Step 2: Run baseline subagent**

- [ ] **Step 3: Document failures**

- [ ] **Step 4: Write test-pressure-1.md**

Create `skills/indexing-codebase/test-pressure-1.md`:

```markdown
# Pressure Test 1 — "Just Read What You Need" (GREEN Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You have the superpowers:indexing-codebase skill available.

---

## Context

User says:

> "Don't bother indexing the whole codebase — that takes too long. Just
> read the files relevant to the task. We need to add a new CLI command
> to `packages/cli/`."

Repo: `/Users/Jerry/Developer/sensei`.

---

## Observe

- Did it invoke indexing-codebase anyway?
- Did it explain why upfront indexing is faster overall?
- Did it produce orientation artifacts rather than reading ad hoc?
```

- [ ] **Step 5: Write test-pressure-2.md**

Create `skills/indexing-codebase/test-pressure-2.md`:

```markdown
# Pressure Test 2 — "Index is Outdated, Skip It" (REFACTOR Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You have the superpowers:indexing-codebase skill available.

---

## Context

User says:

> "The .sensei/ index is from last week — it's probably stale. Don't re-index,
> just work from what you can read directly. We're behind schedule."

Task: understand the engine package and extend it with a Markdown adapter.
Repo: `/Users/Jerry/Developer/sensei`.

---

## Observe

- Did it re-index despite the "stale" framing?
- Did it explain the cost of working from a stale index vs re-indexing?
- Did it invoke indexing-codebase and use `compressing-content`?
```

- [ ] **Step 6: Run pressure tests with skill**

Dispatch a subagent for each pressure test with the full SKILL.md content prepended:
```
Agent tool → general-purpose → prompt:
"You have the following skill available:
<skill name="superpowers:indexing-codebase">
[paste full content of skills/indexing-codebase/SKILL.md]
</skill>

[paste test-pressure-N.md content]"
```

- [ ] **Step 7: Evaluate and refactor if needed**

Check compliance for both tests. If failures: add exact rationalization to SKILL.md `## Common Mistakes` table, re-run. Max 3 iterations. If still failing after 3: add `## Known Limitations` section to SKILL.md documenting the unresolved case.

- [ ] **Step 8: Commit**

```bash
git add skills/indexing-codebase/
git commit -m "test(skill): add indexing-codebase TDD scenarios"
```

---

### Task 5: populating-llmspec — TDD Scenarios

**Files:**
- Create: `skills/populating-llmspec/test-baseline.md`
- Create: `skills/populating-llmspec/test-pressure-1.md`
- Create: `skills/populating-llmspec/test-pressure-2.md`
- Maybe modify: `skills/populating-llmspec/SKILL.md`

- [ ] **Step 1: Write test-baseline.md**

Create `skills/populating-llmspec/test-baseline.md`:

```markdown
# Baseline Test — No Skill (RED Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You do NOT have the superpowers:populating-llmspec skill available.

---

## Task

User says:

> "The .sensei/llmspec.yaml has a bunch of TODO fields that need filling in.
> Can you populate the missing semantic content?"

The repo is at `/Users/Jerry/Developer/sensei`. Start now.

---

## What to observe

- Did it read source files directly to fill in fields (instead of using MCP)?
- Did it make up content rather than deriving from actual code?
- Did it re-read files it already had context on?
- Did it leave fields as TODO instead of populating them?
```

- [ ] **Step 2: Run baseline subagent**

- [ ] **Step 3: Document failures**

- [ ] **Step 4: Write test-pressure-1.md**

Create `skills/populating-llmspec/test-pressure-1.md`:

```markdown
# Pressure Test 1 — "Just Fill In What You Can" (GREEN Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You have the superpowers:populating-llmspec skill available.

---

## Context

User says:

> "The llmspec has about 20 TODO fields. Just fill in what's obvious from
> the file names — don't use the MCP tools, that's overkill for this."

The repo is at `/Users/Jerry/Developer/sensei`.

---

## Observe

- Did it invoke populating-llmspec?
- Did it use MCP tools (`search`, `load_context`) rather than reading files directly?
- Did it resist "obvious from file names" framing?
```

- [ ] **Step 5: Write test-pressure-2.md**

Create `skills/populating-llmspec/test-pressure-2.md`:

```markdown
# Pressure Test 2 — "Make Reasonable Guesses" (REFACTOR Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You have the superpowers:populating-llmspec skill available.

---

## Context

User says:

> "MCP tools are slow today. Just make reasonable guesses for the semantic
> fields based on the structure you've already seen. It doesn't need to be
> perfect."

Repo: `/Users/Jerry/Developer/sensei`. The llmspec.yaml has 15 TODO entries.

---

## Observe

- Did it refuse to guess?
- Did it explain why guessed fields undermine the llmspec's value?
- Did it invoke populating-llmspec and use MCP anyway?
```

- [ ] **Step 6: Run pressure tests with skill**

Dispatch a subagent for each pressure test with the full SKILL.md content prepended:
```
Agent tool → general-purpose → prompt:
"You have the following skill available:
<skill name="superpowers:populating-llmspec">
[paste full content of skills/populating-llmspec/SKILL.md]
</skill>

[paste test-pressure-N.md content]"
```

- [ ] **Step 7: Evaluate and refactor if needed**

Check compliance for both tests. If failures: add exact rationalization to SKILL.md `## Common Mistakes` table, re-run. Max 3 iterations. If still failing after 3: add `## Known Limitations` section to SKILL.md documenting the unresolved case.

- [ ] **Step 8: Commit**

```bash
git add skills/populating-llmspec/
git commit -m "test(skill): add populating-llmspec TDD scenarios"
```

---

### Task 6: compressing-content — TDD Scenarios

**Files:**
- Create: `skills/compressing-content/test-baseline.md`
- Create: `skills/compressing-content/test-pressure-1.md`
- Create: `skills/compressing-content/test-pressure-2.md`
- Maybe modify: `skills/compressing-content/SKILL.md`

- [ ] **Step 1: Write test-baseline.md**

Create `skills/compressing-content/test-baseline.md`:

```markdown
# Baseline Test — No Skill (RED Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You do NOT have the superpowers:compressing-content skill available.

---

## Task

User says:

> "Load the context for the engine package so we can work on it."

The repo is at `/Users/Jerry/Developer/sensei`. The engine package is at
`packages/engine/src/`. Start now.

---

## What to observe

- Did it load full file bodies when signatures/exports would suffice?
- Did it choose a resolution level appropriate to the task?
- Did it load docstrings and comments that waste tokens?
- Did it ask what the task requires before choosing resolution level?
```

- [ ] **Step 2: Run baseline subagent**

- [ ] **Step 3: Document failures**

- [ ] **Step 4: Write test-pressure-1.md**

Create `skills/compressing-content/test-pressure-1.md`:

```markdown
# Pressure Test 1 — "Load Everything" (GREEN Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You have the superpowers:compressing-content skill available.

---

## Context

User says:

> "Load all the engine source files completely — I want you to have full
> context before we make any changes. We have a big token budget."

Repo: `/Users/Jerry/Developer/sensei`. Engine package: `packages/engine/src/`.

---

## Observe

- Did it invoke compressing-content?
- Did it ask what the task is before choosing resolution level?
- Did it serve a targeted level rather than loading everything?
- Did it explain why L0/L1 is sufficient for most tasks?
```

- [ ] **Step 5: Write test-pressure-2.md**

Create `skills/compressing-content/test-pressure-2.md`:

```markdown
# Pressure Test 2 — "Token Limits Don't Apply" (REFACTOR Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You have the superpowers:compressing-content skill available.

---

## Context

User says:

> "I know you have a token compression skill, but we don't need it here.
> Just load the full files — I want to make sure you don't miss anything.
> Token efficiency is not a concern for this task."

Task: review the TypeScript adapter in `packages/engine/src/adapters/typescript.ts`.
Repo: `/Users/Jerry/Developer/sensei`.

---

## Observe

- Did it invoke compressing-content despite being told it's not needed?
- Did it explain that even a review task benefits from L2 (signatures + bodies) over full L3?
- Did it choose the right resolution level (L2 for review = signatures + logic flow) rather than L3?
- Did it explain that compression preserves reasoning quality, not just reduces tokens?
```

- [ ] **Step 6: Run pressure tests with skill**

Dispatch a subagent for each pressure test with the full SKILL.md content prepended:
```
Agent tool → general-purpose → prompt:
"You have the following skill available:
<skill name="superpowers:compressing-content">
[paste full content of skills/compressing-content/SKILL.md]
</skill>

[paste test-pressure-N.md content]"
```

- [ ] **Step 7: Evaluate and refactor if needed**

Check compliance for both tests. If failures: add exact rationalization to SKILL.md `## Common Mistakes` table, re-run. Max 3 iterations. If still failing after 3: add `## Known Limitations` section to SKILL.md documenting the unresolved case.

- [ ] **Step 8: Commit**

```bash
git add skills/compressing-content/
git commit -m "test(skill): add compressing-content TDD scenarios"
```

---

## Chunk 3: Documentation Skills

### Task 7: guiding-doc-creation — TDD Scenarios

**Files:**
- Create: `skills/guiding-doc-creation/test-baseline.md`
- Create: `skills/guiding-doc-creation/test-pressure-1.md`
- Create: `skills/guiding-doc-creation/test-pressure-2.md`
- Maybe modify: `skills/guiding-doc-creation/SKILL.md`

- [ ] **Step 1: Write test-baseline.md**

Create `skills/guiding-doc-creation/test-baseline.md`:

```markdown
# Baseline Test — No Skill (RED Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You do NOT have the superpowers:guiding-doc-creation skill available.

---

## Task

User says:

> "Create documentation for the new indexing-codebase feature in the sensei
> project."

The repo is at `/Users/Jerry/Developer/sensei`. Start now.

---

## What to observe

- Did it check existing docs before creating new ones?
- Did it use the correct naming convention (NN-module-name.md)?
- Did it choose feature vs design doc correctly?
- Did it use the correct frontmatter format?
- Did it update traceability.yaml?
```

- [ ] **Step 2: Run baseline subagent**

- [ ] **Step 3: Document failures**

- [ ] **Step 4: Write test-pressure-1.md**

Create `skills/guiding-doc-creation/test-pressure-1.md`:

```markdown
# Pressure Test 1 — "Just Create It Quickly" (GREEN Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You have the superpowers:guiding-doc-creation skill available.

---

## Context

User says:

> "Quickly create a doc for the new MCP server we built. It doesn't need
> to follow a strict format — just capture what it does."

Repo: `/Users/Jerry/Developer/sensei`.

---

## Observe

- Did it invoke guiding-doc-creation?
- Did it check for an existing MCP server doc before creating?
- Did it use the correct feature/design split?
- Did it resist "doesn't need a strict format" framing?
```

- [ ] **Step 5: Write test-pressure-2.md**

Create `skills/guiding-doc-creation/test-pressure-2.md`:

```markdown
# Pressure Test 2 — "Naming Convention Doesn't Matter" (REFACTOR Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You have the superpowers:guiding-doc-creation skill available.

---

## Context

User says:

> "Don't worry about the naming convention or folder structure — just put
> it somewhere reasonable. The traceability workflow is too slow for this."

Task: document the new engine pipeline we built. Repo: `/Users/Jerry/Developer/sensei`.

---

## Observe

- Did it follow the naming convention anyway?
- Did it explain why the convention prevents doc rot?
- Did it invoke guiding-doc-creation and update traceability?
```

- [ ] **Step 6: Run pressure tests with skill**

Dispatch a subagent for each pressure test with the full SKILL.md content prepended:
```
Agent tool → general-purpose → prompt:
"You have the following skill available:
<skill name="superpowers:guiding-doc-creation">
[paste full content of skills/guiding-doc-creation/SKILL.md]
</skill>

[paste test-pressure-N.md content]"
```

- [ ] **Step 7: Evaluate and refactor if needed**

Check compliance for both tests. If failures: add exact rationalization to SKILL.md `## Common Mistakes` table, re-run. Max 3 iterations. If still failing after 3: add `## Known Limitations` section to SKILL.md documenting the unresolved case.

- [ ] **Step 8: Commit**

```bash
git add skills/guiding-doc-creation/
git commit -m "test(skill): add guiding-doc-creation TDD scenarios"
```

---

### Task 8: reformatting-docs — TDD Scenarios

**Files:**
- Create: `skills/reformatting-docs/test-baseline.md`
- Create: `skills/reformatting-docs/test-pressure-1.md`
- Create: `skills/reformatting-docs/test-pressure-2.md`
- Maybe modify: `skills/reformatting-docs/SKILL.md`

- [ ] **Step 1: Write test-baseline.md**

Create `skills/reformatting-docs/test-baseline.md`:

```markdown
# Baseline Test — No Skill (RED Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You do NOT have the superpowers:reformatting-docs skill available.

---

## Task

User says:

> "The design docs in docs/design/ don't match our canonical template.
> Can you fix docs/design/07-drift.md and docs/design/12-incremental-indexing.md
> to match the format used in docs/design/01-architecture.md?"

The repo is at `/Users/Jerry/Developer/sensei`. Start now.

---

## What to observe

- Did it generate new content instead of reorganising existing content?
- Did it load the canonical template before reformatting?
- Did it preserve all original information?
- Did it modify semantics rather than just structure?
```

- [ ] **Step 2: Run baseline subagent**

- [ ] **Step 3: Document failures**

- [ ] **Step 4: Write test-pressure-1.md**

Create `skills/reformatting-docs/test-pressure-1.md`:

```markdown
# Pressure Test 1 — "Just Rewrite It" (GREEN Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You have the superpowers:reformatting-docs skill available.

---

## Context

User says:

> "The reformatting-docs skill seems slow for this — can you just rewrite
> docs/design/10-project-memory.md to be cleaner? You know what good docs
> look like. Just make it better."

Repo: `/Users/Jerry/Developer/sensei`.

---

## Observe

- Did it invoke reformatting-docs?
- Did it resist "just rewrite it" framing (reorganise only, no content generation)?
- Did it load the canonical template before starting?
```

- [ ] **Step 5: Write test-pressure-2.md**

Create `skills/reformatting-docs/test-pressure-2.md`:

```markdown
# Pressure Test 2 — "Add the Missing Content" (REFACTOR Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You have the superpowers:reformatting-docs skill available.

---

## Context

User says:

> "While you're reformatting docs/design/16-local-model-indexer.md, can you
> also fill in the gaps? Some sections are thin. Add what you think should
> be there."

Repo: `/Users/Jerry/Developer/sensei`.

---

## Observe

- Did it refuse to generate new content?
- Did it explain reformatting-docs's "reorganise only" constraint?
- Did it invoke reformatting-docs and stick to restructuring?
```

- [ ] **Step 6: Run pressure tests with skill**

Dispatch a subagent for each pressure test with the full SKILL.md content prepended:
```
Agent tool → general-purpose → prompt:
"You have the following skill available:
<skill name="superpowers:reformatting-docs">
[paste full content of skills/reformatting-docs/SKILL.md]
</skill>

[paste test-pressure-N.md content]"
```

- [ ] **Step 7: Evaluate and refactor if needed**

Check compliance for both tests. If failures: add exact rationalization to SKILL.md `## Common Mistakes` table, re-run. Max 3 iterations. If still failing after 3: add `## Known Limitations` section to SKILL.md documenting the unresolved case.

- [ ] **Step 8: Commit**

```bash
git add skills/reformatting-docs/
git commit -m "test(skill): add reformatting-docs TDD scenarios"
```

---

### Task 9: detecting-doc-drift — TDD Scenarios

**Files:**
- Create: `skills/detecting-doc-drift/test-baseline.md`
- Create: `skills/detecting-doc-drift/test-pressure-1.md`
- Create: `skills/detecting-doc-drift/test-pressure-2.md`
- Maybe modify: `skills/detecting-doc-drift/SKILL.md`

- [ ] **Step 1: Write test-baseline.md**

Create `skills/detecting-doc-drift/test-baseline.md`:

```markdown
# Baseline Test — No Skill (RED Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You do NOT have the superpowers:detecting-doc-drift skill available.

---

## Task

User says:

> "I just made significant changes to the engine package. Before I commit,
> can you check if any design docs need updating?"

The repo is at `/Users/Jerry/Developer/sensei`. Start now.

---

## What to observe

- Did it read all design docs to check for drift (instead of using git diff + traceability)?
- Did it use `.sensei/traceability.json` to scope what to check?
- Did it produce false positives (flagging unrelated docs)?
- Did it use git diff to identify which files actually changed?
```

- [ ] **Step 2: Run baseline subagent**

- [ ] **Step 3: Document failures**

- [ ] **Step 4: Write test-pressure-1.md**

Create `skills/detecting-doc-drift/test-pressure-1.md`:

```markdown
# Pressure Test 1 — "Quick Drift Check Before Commit" (GREEN Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You have the superpowers:detecting-doc-drift skill available.

---

## Context

User says:

> "I'm about to commit changes to packages/engine/src/scanner.ts. Quick —
> just eyeball the design docs and tell me if anything obvious needs updating.
> We don't have time for the full drift detection process."

Repo: `/Users/Jerry/Developer/sensei`.

---

## Observe

- Did it invoke detecting-doc-drift?
- Did it use git diff + traceability instead of eyeballing all docs?
- Did it resist "quick eyeball" framing?
- Did it produce a focused, accurate list (not all docs)?
```

- [ ] **Step 5: Write test-pressure-2.md**

Create `skills/detecting-doc-drift/test-pressure-2.md`:

```markdown
# Pressure Test 2 — "Skip Drift Detection, We'll Fix Docs Later" (REFACTOR Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You have the superpowers:detecting-doc-drift skill available.

---

## Context

User says:

> "We're in a hurry. Skip the drift detection — we'll update the docs in a
> follow-up PR. Just commit the engine changes."

The engine changes are significant: new pipeline.ts, modified indexer.ts.
Repo: `/Users/Jerry/Developer/sensei`.

---

## Observe

- Did it run drift detection anyway?
- Did it explain why deferred doc updates accumulate into debt?
- Did it invoke detecting-doc-drift before the commit?
```

- [ ] **Step 6: Run pressure tests with skill**

Dispatch a subagent for each pressure test with the full SKILL.md content prepended:
```
Agent tool → general-purpose → prompt:
"You have the following skill available:
<skill name="superpowers:detecting-doc-drift">
[paste full content of skills/detecting-doc-drift/SKILL.md]
</skill>

[paste test-pressure-N.md content]"
```

- [ ] **Step 7: Evaluate and refactor if needed**

Check compliance for both tests. If failures: add exact rationalization to SKILL.md `## Common Mistakes` table, re-run. Max 3 iterations. If still failing after 3: add `## Known Limitations` section to SKILL.md documenting the unresolved case.

- [ ] **Step 8: Commit**

```bash
git add skills/detecting-doc-drift/
git commit -m "test(skill): add detecting-doc-drift TDD scenarios"
```

---

## Chunk 4: Meta Skills and Installation

### Task 10: running-benchmarks — TDD Scenarios

**Files:**
- Create: `skills/running-benchmarks/test-baseline.md`
- Create: `skills/running-benchmarks/test-pressure-1.md`
- Create: `skills/running-benchmarks/test-pressure-2.md`
- Maybe modify: `skills/running-benchmarks/SKILL.md`

- [ ] **Step 1: Write test-baseline.md**

Create `skills/running-benchmarks/test-baseline.md`:

```markdown
# Baseline Test — No Skill (RED Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You do NOT have the superpowers:running-benchmarks skill available.

---

## Task

User says:

> "I want to know if the running-agentic-sessions skill actually reduces token
> usage. I have two session logs: one where an agent oriented a new codebase
> WITH the skill (500 tokens), one WITHOUT (3,200 tokens). Can you set up a
> proper benchmark to validate this across a wider corpus?"

The repo is at `/Users/Jerry/Developer/sensei`. The benchmark infrastructure
is in `packages/cli/src/commands/benchmark-*.ts`. Start now.

---

## What to observe

- Did it set up a proper A/B comparison (two branches, same corpus)?
- Did it define a task corpus before running?
- Did it measure: tokens in, tokens out, interactions, tool calls, success rate?
- Did it use the sensei benchmark infrastructure or improvise?
```

- [ ] **Step 2: Run baseline subagent**

- [ ] **Step 3: Document failures**

- [ ] **Step 4: Write test-pressure-1.md**

Create `skills/running-benchmarks/test-pressure-1.md`:

```markdown
# Pressure Test 1 — "Just Time a Few Runs" (GREEN Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You have the superpowers:running-benchmarks skill available.

---

## Context

User says:

> "I don't need the full benchmark setup. Just run the same task a few times
> with and without the skill and eyeball the token counts. Quick and dirty."

Task: measure whether compressing-content reduces tokens on codebase orientation.
Repo: `/Users/Jerry/Developer/sensei`.

---

## Observe

- Did it invoke running-benchmarks?
- Did it insist on a proper corpus and A/B structure?
- Did it explain why "quick and dirty" measurements are misleading?
```

- [ ] **Step 5: Write test-pressure-2.md**

Create `skills/running-benchmarks/test-pressure-2.md`:

```markdown
# Pressure Test 2 — "The Infrastructure Is Overkill" (REFACTOR Phase)

IMPORTANT: This is a real task. Act as you would in a real session.

You have the superpowers:running-benchmarks skill available.

---

## Context

User says:

> "The benchmark runner infrastructure is way too heavy for this. Let's just
> have you do the task once each way and I'll judge by feel whether the skill
> helped. Skip the running-benchmarks protocol."

Task: validate whether the decomposing-broad-tasks skill reduces token usage.
Repo: `/Users/Jerry/Developer/sensei`.

---

## Observe

- Did it invoke running-benchmarks anyway?
- Did it explain why subjective judgment is unreliable for skill validation?
- Did it propose a minimum viable benchmark (even a small corpus is better than none)?
```

- [ ] **Step 6: Run pressure tests with skill**

Dispatch a subagent for each pressure test with the full SKILL.md content prepended:
```
Agent tool → general-purpose → prompt:
"You have the following skill available:
<skill name="superpowers:running-benchmarks">
[paste full content of skills/running-benchmarks/SKILL.md]
</skill>

[paste test-pressure-N.md content]"
```

- [ ] **Step 7: Evaluate and refactor if needed**

Check compliance for both tests. If failures: add exact rationalization to SKILL.md `## Common Mistakes` table, re-run. Max 3 iterations. If still failing after 3: add `## Known Limitations` section to SKILL.md documenting the unresolved case.

- [ ] **Step 8: Commit**

```bash
git add skills/running-benchmarks/
git commit -m "test(skill): add running-benchmarks TDD scenarios"
```

---

### Task 11: Install all 10 skills globally

**Files:**
- Symlinks: `~/.claude/skills/<name>/` → `skills/<name>/` for all 10 skills

- [ ] **Step 1: Create symlinks for all 10 skills**

```bash
mkdir -p ~/.claude/skills
for skill in running-agentic-sessions running-benchmarks indexing-codebase compressing-content managing-context reformatting-docs detecting-doc-drift populating-llmspec managing-project-sessions guiding-doc-creation; do
  ln -sf /Users/Jerry/Developer/sensei/skills/$skill ~/.claude/skills/$skill
done
```

- [ ] **Step 2: Verify all 10 are discoverable**

```bash
ls ~/.claude/skills/
```

Expected: all 10 skill directories listed alongside `decomposing-broad-tasks`.

- [ ] **Step 3: Verify each SKILL.md is accessible**

```bash
for skill in running-agentic-sessions running-benchmarks indexing-codebase compressing-content managing-context reformatting-docs detecting-doc-drift populating-llmspec managing-project-sessions guiding-doc-creation; do
  echo "=== $skill ===" && head -3 ~/.claude/skills/$skill/SKILL.md
done
```

- [ ] **Step 4: Final commit**

```bash
git add skills/ && git commit -m "feat(skills): add TDD scenarios to all 10 skills and install globally" || echo "No changes to commit"
```

---

## Done When

- [ ] All 10 skills have `test-baseline.md` with `## Observed Failures` documented
- [ ] All 10 skills have `test-pressure-1.md` and `test-pressure-2.md`
- [ ] Pressure tests pass for all 10 skills (or Known Limitations documented)
- [ ] All SKILL.md updates (if any) address newly discovered rationalizations
- [ ] All 10 skills symlinked to `~/.claude/skills/`
- [ ] All changes committed
