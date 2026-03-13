---
id: decomposing-broad-tasks
type: spec
status: approved
date: 2026-03-13
---

# Decomposing Broad Tasks — Design Spec

## Goal

A superpowers skill that intercepts broad multi-file requests before any bulk reading occurs, decomposes them into focused, context-minimal agent chunks, and dispatches those agents with exactly the shared context they need. Reduces token waste in the coordinating context while improving agent task quality.

---

## Problem

When faced with a broad request ("refactor all 18 docs", "update every feature file"), the default pattern is:

1. Read many files into the coordinating context to understand scope
2. Dispatch agents that re-read those same files
3. Each agent carries shared context that was re-derived independently

This compounds token cost: coordinating context bloats, agents duplicate work, shared context is never extracted cleanly.

---

## Design

### Name

`decomposing-broad-tasks`

Follows superpowers verb-phrase naming convention.

### Trigger — When Claude Self-Invokes

```
- User request touches 5+ files OR mentions "all", "every", "refactor all",
  "update all", "clean up all", "audit all"
  (below 5 files the decomposition overhead exceeds the benefit — handle inline)
- OR: Claude is about to Glob a pattern that will return 5+ results and
  then read most of them
- OR: Claude has started reading files sequentially with no clear stopping
  condition (fallback trigger — retrospective, less ideal than the above two)
```

### Two Operating Modes

| Mode | How | When |
|---|---|---|
| **Self-recognition** (Phase A) | Claude detects broadness from conversation context, invokes skill | Always — the baseline |
| **Hook-assisted** (Phase B) | `UserPromptSubmit` hook pre-classifies with local model, injects hint | Ollama running + hook installed |

Both modes run the same decomposition protocol. The hook moves classification earlier (before any tools fire) and uses a cheaper model for it.

**What this skill is NOT:**
- Not a replacement for `writing-plans` (that's for implementation work with TDD steps)
- Not a replacement for `dispatching-parallel-agents` (that's dispatch mechanics)
- It is the missing step *between* recognising a broad request and dispatching: **deciding how to slice it**

---

## Decomposition Protocol

Five steps, executed before any bulk file reading.

### Step 1 — Lightweight Scan

Read only what is needed to understand scope: filenames, frontmatter, first ~10 lines. For docs: `id`, `type`, overview sentence. For code: exports, imports. The goal is a map of what exists, not content.

**Hard rule: do not read file bodies during steps 1–3.**

### Step 2 — Identify Shared Context

Extract the briefing that every agent will need regardless of which files it touches. Write it as a compact brief (~100–200 tokens). This is extracted once and passed to every agent — never re-derived per agent.

Examples of shared context:
- Architecture constraint: "All `.index/` file references → Supabase tables"
- Style rule: "Remove implementation specifics, keep user-observable behaviour only"
- Pattern: "Follow the existing frontmatter format in every file"

### Step 3 — Cluster by Independence

Group files/tasks that can be completed without knowledge of each other.

| Cluster type | Rule | Dispatch |
|---|---|---|
| **Independent** | No cross-references, no shared output | Parallel |
| **Ordered** | Output of one is input of another | Sequential |
| **Coupled** | Deeply intertwined — cannot be separated cleanly | Single agent |

Do not force-split coupled work. A 3-file coupled cluster is better handled by one agent than three agents guessing at each other's changes.

### Step 4 — Scope Each Agent Brief

For each cluster, produce a brief using the Agent Brief Format below (four required sections, one optional). No pre-loaded file content — the agent reads only what it needs.

If Step 3 produces a single cluster covering all files, the task is not broad enough to decompose — skip to direct execution without dispatching sub-agents.

### Step 5 — Dispatch and Integrate

Hand off to `dispatching-parallel-agents` for the actual dispatch mechanics. Independent clusters dispatch in parallel; ordered clusters dispatch sequentially.

When agents return, the coordinating agent receives their summaries and presents a consolidated result to the user — it does not re-read the files agents touched.

---

## Agent Brief Format

```markdown
## Shared Context
<architecture note / key constraint that applies to all agents>
~100-200 tokens max

## Your Scope
Files: [exact list]
Goal: [one sentence]
Constraints: [what NOT to do]

## Reference
<optional: pointer to a doc or pattern to follow — not pre-loaded content>

## Return
<what the agent should summarize when done>
```

**Concrete example:**

```markdown
## Shared Context
Architecture change: all indexed data now lives in Supabase (sensei.symbols,
sensei.scan_state, sensei.events, sensei.repos). Replace all .index/ file
references with Supabase equivalents. Generated orientation files live at
.sensei/ (not .index/).

## Your Scope
Files: [07-drift.md, 12-incremental-indexing.md, 13-traceability-matrix.md]
Goal: Update storage references in these three docs
Constraints: Do not change algorithms, NFRs, or CLI behaviour descriptions

## Reference
See 40-metadata-model.md for the Supabase schema

## Return
List of specific replacements made per file
```

**What this prevents:**
- Agent reading files outside its scope
- Shared context re-derived independently per agent
- Coordinating context accumulating full file content
- Agents making changes outside their assigned scope

---

## Phase B: Hook Design

The hook makes the skill automatic — classification happens before Claude touches any files.

**Hook type:** `UserPromptSubmit`
**Fires:** Once per user message, before any tools
**Input:** Full user prompt text
**Output:** JSON `additionalContext` injected into Claude's conversation

### Flow

```
User submits message
        ↓
UserPromptSubmit hook fires
        ↓
sensei decompose --prompt "<message>"
        ↓
Local model (llama3.2:3b) classifies:
  - Is this a broad multi-file task? (yes/no)
  - If yes: estimated file count, task type (refactor/update/audit/create)
        ↓
If broad → inject into context:
  <decomposition-hint>
  Broad task detected: ~18 files, type: doc-refactor
  Invoke superpowers:decomposing-broad-tasks before reading any files.
  </decomposition-hint>
        ↓
If not broad → pass through, zero overhead
```

### Local Model Prompt

Minimal: user message + classify instruction. No file content. Response is a JSON classification:

```json
{ "broad": true, "fileCount": 18, "type": "doc-refactor" }
```

`fileCount` is advisory — used to inform cluster sizing, not enforced. `type` is one of: `refactor`, `update`, `audit`, `generate`, `unknown`.

Latency: ~200ms on a warm Ollama instance.

### Graceful Degradation

| Condition | Behaviour |
|---|---|
| Ollama not running | Hook exits cleanly, no injection, skill self-recognition takes over |
| `sensei` CLI not installed | Same — hook is advisory, not required |
| False positive classification | Claude reads hint, sees it does not apply, proceeds normally |
| False negative (broad task missed) | Skill self-recognition catches it |

### Phase B Additions to Sensei

- `sensei decompose --prompt <text>` CLI command — thin wrapper around `packages/engine/` + `ModelBackend`
- Hook registration in plugin `hooks.json` under `UserPromptSubmit`
- Skill gains a "hook-assisted mode" section describing how to consume the injected hint

---

## Relationship to Existing Skills

```
superpowers:decomposing-broad-tasks   ← this skill: decide HOW to slice
        ↓
superpowers:dispatching-parallel-agents  ← dispatch mechanics
        ↓
agents execute their focused briefs
```

`writing-plans` is orthogonal — it decomposes *implementation work* (TDD steps, commits). This skill decomposes *any broad request* (refactoring, auditing, updating, generating).

---

## Non-Functional Requirements

| NFR | Requirement |
|---|---|
| token-efficiency | Coordinating context must not hold full file bodies during decomposition |
| reliability | Skill works without hook; hook is an enhancement only |
| accuracy | Coupled work must not be force-split; clustering must err toward fewer, larger agents over many tiny ones |
| latency (Phase B) | Hook classification must add under 500ms to message submission |
