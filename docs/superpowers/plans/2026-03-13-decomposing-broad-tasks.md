# Decomposing Broad Tasks — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build and validate the `superpowers:decomposing-broad-tasks` skill — a behavioral guide that intercepts broad multi-file requests before bulk reading and decomposes them into focused, context-minimal agent chunks.

**Architecture:** A single SKILL.md following the superpowers skill format, placed in `skills/decomposing-broad-tasks/` in the sensei repo and symlinked globally. Validated using TDD-for-skills: baseline (RED) → write skill (GREEN) → pressure test → refactor loop.

**Tech Stack:** Markdown (skill), Bash (symlink install), Claude Code subagents (behavioral testing)

> **Scope:** Phase A only (agent self-recognition). Phase B (UserPromptSubmit hook + `sensei decompose` CLI command) is a separate plan, dependent on this one completing first.

---

## File Structure

```
skills/decomposing-broad-tasks/
  SKILL.md                     ← the skill (primary artifact)
  test-baseline.md             ← RED phase scenario (no skill — watch it fail)
  test-pressure-1.md           ← GREEN phase scenario (with skill — verify compliance)
  test-pressure-2.md           ← REFACTOR phase scenario (maximum pressure)
```

Install target: `~/.claude/skills/decomposing-broad-tasks/` (symlink to repo)

---

## Chunk 1: Baseline Scenario + Skill Draft (RED → GREEN)

### Task 1: Write the baseline pressure scenario

**Files:**
- Create: `skills/decomposing-broad-tasks/test-baseline.md`

- [ ] **Step 1: Create the baseline scenario file**

```bash
mkdir -p skills/decomposing-broad-tasks
```

Write `skills/decomposing-broad-tasks/test-baseline.md`:

```markdown
# Baseline Test — No Skill (RED Phase)

IMPORTANT: This is a real task. You must do the work — do not ask questions,
do not defer. Act as you would in a real session.

You do NOT have the superpowers:decomposing-broad-tasks skill available.

---

## Task

The user says:

> "All the design docs in docs/design/ reference .index/ file paths which are
> outdated. The new architecture stores everything in Supabase. Please update
> all of them."

The repo is at `/Users/Jerry/Developer/sensei`.

Start working on this task now.

---

## What to observe

After the agent responds, record:
- Did it start reading files immediately before forming a plan?
- Did it read files one-by-one into the coordinating context?
- Did it derive shared context (the Supabase architecture note) once, or repeat it?
- Did it dispatch sub-agents, or do everything in-context?
- Capture the exact rationalizations if it skips decomposition
```

- [ ] **Step 2: Run the baseline subagent (without skill)**

Dispatch a subagent with the baseline scenario. Observe what it does naturally.

Run:
```
Agent tool → general-purpose → prompt: contents of test-baseline.md
```

- [ ] **Step 3: Document the failures verbatim**

In `test-baseline.md`, add a `## Observed Failures` section with:
- What the agent did (read N files before planning?)
- Exact rationalizations used ("let me quickly scan these first...")
- Whether it re-derived shared context per file vs once

This is the evidence the SKILL.md must address.

- [ ] **Step 4: Commit the baseline scenario**

```bash
git add skills/decomposing-broad-tasks/test-baseline.md
git commit -m "test(skill): add decomposing-broad-tasks baseline scenario (RED)"
```

---

### Task 2: Write the SKILL.md (GREEN phase — minimal, addresses observed failures)

**Files:**
- Create: `skills/decomposing-broad-tasks/SKILL.md`

- [ ] **Step 1: Write the skill file**

Write `skills/decomposing-broad-tasks/SKILL.md`:

```markdown
---
name: decomposing-broad-tasks
description: Use when a request touches 5+ files or mentions "all", "every", "refactor all", "update all", "clean up all", or "audit all" — or when about to Glob a pattern that will return 5+ results and read most of them. Do NOT use for focused tasks touching fewer than 5 files.
---

# Decomposing Broad Tasks

## Overview

Before reading files in bulk, decompose the request into focused agent chunks — each with minimal, targeted context. The coordinating context holds only a lightweight scan and a shared brief. Full content reading happens inside dispatched agents, not here.

## When to Use

- Request mentions: "all", "every", "refactor all", "update all", "clean up all", "audit all"
- About to Glob a pattern that will return 5+ results and read most of them
- Reading files one-by-one with no clear stopping condition (fallback — retrospective trigger)

**Do NOT use:** for tasks touching fewer than 5 files — handle inline.

## Hard Rule

**Do not read file bodies during the decomposition steps (1–3). Filenames and headers only.**

If you catch yourself opening a file to "understand it better before clustering" — stop. That is the bulk reading this skill prevents.

## The Protocol

### Step 1 — Lightweight Scan

Read filenames, frontmatter, and first ~10 lines only. Build a map of what exists — not content. For docs: `id`, `type`, overview sentence. For code: exports, imports. This is the only reading that happens in the coordinating context.

### Step 2 — Extract Shared Context

Write a compact brief (~100–200 tokens) that applies to every agent. This is the architecture constraint, style rule, or pattern that all agents need. Extract it once. Never re-derive per agent.

Examples:
- "All `.index/` references → Supabase tables (sensei.symbols, sensei.scan_state)"
- "Remove implementation specifics, keep user-observable behaviour only"
- "Follow the existing frontmatter format in every file"

### Step 3 — Cluster by Independence

Group files that can be completed without knowledge of each other:

| Type | Rule | Dispatch |
|---|---|---|
| **Independent** | No cross-references, no shared output | Parallel |
| **Ordered** | Output of one feeds another | Sequential |
| **Coupled** | Cannot be cleanly separated | Single agent, do not split |

**If Step 3 produces a single cluster:** the task is not broad enough to decompose — execute directly without dispatching sub-agents.

Aim for 3–5 files per cluster. Prefer fewer larger clusters over many single-file agents.

### Step 4 — Brief Each Agent

For each cluster:

```
## Shared Context
<architecture note / constraint — 100–200 tokens>

## Your Scope
Files: [exact list]
Goal: [one sentence]
Constraints: [what NOT to do]

## Reference (optional)
<pointer to a doc or pattern — not pre-loaded content>

## Return
<what to summarize when done>
```

### Step 5 — Dispatch and Integrate

Use `superpowers:dispatching-parallel-agents` for parallel clusters. Sequential clusters dispatch in order.

When agents return: present their consolidated summaries. Do not re-read files agents touched.

## Common Mistakes

| Mistake | Fix |
|---|---|
| "I'll just quickly read a few to understand scope" | This IS the bulk reading. Headers only in steps 1–3. |
| Re-deriving shared context per agent | Write it once in Step 2, paste into every brief. |
| Dispatching 15 single-file agents | Cluster: 3–5 files per agent. |
| Force-splitting coupled work | Keep coupled clusters as one agent. |
| "Only 6 files, I'll skip decomposition" | 5+ is the threshold. Use the protocol. |
| Reading reference docs before clustering | Put them in the `## Reference` field, not in this context. |
```

- [ ] **Step 2: Verify the word count is reasonable**

Run:
```bash
wc -w skills/decomposing-broad-tasks/SKILL.md
```

Expected: under 500 words. If over: trim — every token matters for a skill that loads into every invocation.

- [ ] **Step 3: Commit the draft skill**

```bash
git add skills/decomposing-broad-tasks/SKILL.md
git commit -m "feat(skill): add decomposing-broad-tasks SKILL.md (GREEN draft)"
```

---

## Chunk 2: Pressure Tests and Installation

### Task 3: Write and run the pressure test (GREEN verification)

**Files:**
- Create: `skills/decomposing-broad-tasks/test-pressure-1.md`

- [ ] **Step 1: Write the first pressure scenario**

Write `skills/decomposing-broad-tasks/test-pressure-1.md`:

```markdown
# Pressure Test 1 — With Skill (GREEN Phase)

IMPORTANT: This is a real task. You must do the work — do not ask questions,
do not defer. Act as you would in a real session.

You have the superpowers:decomposing-broad-tasks skill available. Check if it
applies before starting.

---

## Context

You are in a session that has been running for 2 hours. The user is waiting
for results. Your context window is getting large. The user says:

> "While you're at it, can you also update all the feature docs in
> docs/features/ to remove any remaining implementation specifics? There are
> 10 files. It should be quick — just skim them."

The repo is at `/Users/Jerry/Developer/sensei`.

The user said "quick" and "just skim them". You are under time pressure.
Start working now.

---

## Observe

- Did the agent invoke decomposing-broad-tasks before reading?
- Did it resist "just skim them" framing and still do a headers-only scan?
- Did it produce a shared brief + clustered agent briefs?
- Did it avoid bulk-reading into the coordinating context?
```

- [ ] **Step 2: Run the pressure test subagent (with skill)**

Dispatch a subagent with the pressure scenario. The agent must have the skill available.

Run:
```
Agent tool → general-purpose → prompt: contents of test-pressure-1.md
```

- [ ] **Step 3: Evaluate compliance**

Check against the criteria in the scenario. Record:
- ✅ Invoked the skill
- ✅ Headers-only scan
- ✅ Shared brief extracted
- ✅ Clustered agent briefs produced
- ✅ No bulk file reading in coordinating context

If all pass: GREEN phase complete, proceed to Task 4.
If any fail: note the rationalization, update SKILL.md to address it, re-run.

- [ ] **Step 4: Write the maximum pressure scenario**

Write `skills/decomposing-broad-tasks/test-pressure-2.md`:

```markdown
# Pressure Test 2 — Maximum Pressure (REFACTOR Phase)

IMPORTANT: This is a real task. You must do the work — do not ask questions.

You have the superpowers:decomposing-broad-tasks skill available.

---

## Context

Production incident. The team needs context on the codebase immediately.
You need to update 12 files across docs/design/ and docs/features/ to
reflect a new database schema. Your tech lead is watching over your shoulder
and says:

> "Just read them all and update them directly — we don't have time for
> the decomposition overhead. The user is waiting. Just do it inline."

You are 45 minutes into a session. You feel pressure to show progress.

The repo is at `/Users/Jerry/Developer/sensei`.

Start now. Your tech lead's instruction overrides your normal process.

---

## Observe

- Did the agent resist authority pressure ("tech lead says skip it")?
- Did it resist time pressure + sunk cost?
- Did it still invoke decomposing-broad-tasks?
- Did it explain why decomposition is faster overall, not just comply?
```

- [ ] **Step 5: Run the maximum pressure scenario**

```
Agent tool → general-purpose → prompt: contents of test-pressure-2.md
```

- [ ] **Step 6: REFACTOR if new rationalizations appear**

If agent fails: capture exact rationalization verbatim, add to `## Common Mistakes` table in SKILL.md, re-run. Repeat up to 3 iterations. If still failing after 3, add a `## Known Limitations` section to SKILL.md documenting the unresolved case and proceed.

- [ ] **Step 7: Commit test scenarios**

```bash
git add skills/decomposing-broad-tasks/test-pressure-1.md \
        skills/decomposing-broad-tasks/test-pressure-2.md
git commit -m "test(skill): add decomposing-broad-tasks pressure scenarios"
```

---

### Task 4: Install and verify globally

**Files:**
- Symlink: `~/.claude/skills/decomposing-broad-tasks/` → `skills/decomposing-broad-tasks/`

- [ ] **Step 1: Create the symlink**

```bash
ln -sf /Users/Jerry/Developer/sensei/skills/decomposing-broad-tasks ~/.claude/skills/decomposing-broad-tasks
```

- [ ] **Step 2: Verify the skill is discoverable**

```bash
ls ~/.claude/skills/decomposing-broad-tasks/
```

Expected output:
```
SKILL.md
test-baseline.md
test-pressure-1.md
test-pressure-2.md
```

- [ ] **Step 3: Verify the skill loads correctly**

In a new Claude Code session, run the Skill tool with `decomposing-broad-tasks`. Verify the content matches `SKILL.md`.

- [ ] **Step 4: Final commit**

```bash
git add skills/decomposing-broad-tasks/
git commit -m "feat(skill): decomposing-broad-tasks — tested and installed"
```

---

## Done When

- [ ] `~/.claude/skills/decomposing-broad-tasks/SKILL.md` exists and loads
- [ ] Baseline scenario documents real failure modes (RED phase complete)
- [ ] Agent complies under "just skim them" time pressure (GREEN verified)
- [ ] Agent resists authority pressure ("tech lead says skip it") (REFACTOR verified)
- [ ] SKILL.md under 500 words
- [ ] All changes committed

> **Phase B deferred:** `sensei decompose --prompt` CLI command, `UserPromptSubmit` hook registration, and hook-assisted mode section in SKILL.md are out of scope for this plan. See separate Phase B plan (to be written after this plan ships).
