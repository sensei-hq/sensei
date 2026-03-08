# populate-llmspec Skill Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create a `populate-llmspec` skill that guides Claude to fill `.sensei/llmspec.yaml` semantic fields (description, entry_points, concepts, patterns, docs[].covers[]) using its own code understanding rather than the local model.

**Architecture:** The existing `sensei index` command generates the structural scaffold (symbol-map.json, llmspec.yaml skeleton with TODO placeholders). The new skill takes over from there — it's a Claude Code skill that reads the repo using MCP tools + file reads, then writes the filled-in llmspec.yaml. The local model / `generateCoverage` tool remains available but the skill approach produces higher quality.

**Tech Stack:** Markdown (skill files only — no TypeScript changes needed)

---

### Task 1: Create `skills/populate-llmspec/SKILL.md`

**Files:**
- Create: `skills/populate-llmspec/SKILL.md`

**Step 1: Create the skill directory**

```bash
mkdir -p skills/populate-llmspec
```

**Step 2: Write the skill file**

Create `skills/populate-llmspec/SKILL.md` with this exact content:

````markdown
---
name: populate-llmspec
description: Use when .sensei/llmspec.yaml has TODO placeholders or an empty docs[] section, when setting up a new repo with sensei, or when llmspec coverage score is below 70/100.
---

# Populate LLMSpec

## Overview

Fill `.sensei/llmspec.yaml` semantic fields using code understanding. `sensei index` builds the structural scaffold; this skill fills in the meaning: what the project does, which patterns it uses, and which docs cover which source files.

## When to Use

- After `sensei init` or `sensei index` generates a fresh llmspec with TODO placeholders
- When `bun tasks/score-coverage.ts .sensei/llmspec.yaml` scores below 70/100
- When docs[].covers[] is empty but docs/ directory has content

## Protocol

**Step 1: Orient**

```
call: get_llmspec()
```

Identify which fields have TODO values or are empty. Note the `docs[]` array — this is the main work.

**Step 2: Fill `description`**

Read `README.md` (first 30 lines). Write one sentence: what the project does and for whom.

**Step 3: Fill `entry_points`**

```
call: list_exports("packages/cli/src/cli.ts")   ← or wherever the main binary lives
```

For each entry point: one phrase describing its role (e.g. "MCP server entry — registers N tools over stdio").

**Step 4: Fill `concepts`**

Read `docs/design/` index or README. Extract 4–6 domain terms that aren't obvious from symbol names. Format: `name` + one-sentence `definition`.

**Step 5: Fill `patterns`**

```
call: find_pattern("export")
call: get_shortcuts()
```

Look for repeated structural conventions (e.g. injectable deps, barrel exports, workspace:* protocol). Format: `name` + `files` glob + `convention` sentence.

**Step 6: Fill `docs[].covers[]`** ← most important, most token-intensive

For each doc file in `docs/`:

1. Read the doc (first 60 lines is usually enough)
2. Ask: which source files does this doc *primarily describe*? Not files it merely mentions — files whose API, algorithm, or design it explains.
3. Use `list_exports(path)` on candidate files to confirm the match
4. Write the covers[] list using exact relative paths from `symbol-map.json`

**Rules for covers[]:**
- Include only files the doc directly documents (direct author intent)
- Exclude: test files, config files, stub files
- Exclude files only tangentially mentioned
- A design doc that describes an algorithm → covers the file implementing it
- A feature doc → covers the tool/command file(s) that implement the feature
- Max ~5 files per doc unless it's a broad overview doc

**Step 7: Write the updated llmspec**

Edit `.sensei/llmspec.yaml` directly. Preserve all existing fields that were already filled (stack, shortcuts, doc_layers). Only replace TODO values and the docs[] array.

**Step 8: Score**

```bash
bun tasks/score-coverage.ts .sensei/llmspec.yaml
```

Target: 70+/100. If below 70, revisit the docs[] entries with the lowest coverage hits.

**Step 9: Commit**

```bash
git add .sensei/llmspec.yaml
git commit -m "chore: populate llmspec — description, concepts, patterns, doc coverage"
```

## Quick Reference

| Field | Source | Effort |
|---|---|---|
| `description` | README.md first paragraph | 30 seconds |
| `entry_points` | package.json `bin` field + list_exports | 2 minutes |
| `concepts` | docs/design/ headings + README glossary | 3 minutes |
| `patterns` | symbol-map L0 patterns + find_pattern | 3 minutes |
| `docs[].covers[]` | read each doc, match to symbol-map | 10–20 minutes |

## Common Mistakes

| Mistake | Fix |
|---|---|
| Mapping a doc to every file it mentions | Only cover files the doc *primarily explains* |
| Including test/spec files in covers[] | Test files are never in covers[] |
| Writing covers[] for plan/template docs | Skip docs/plans/ and docs/templates/ — only feature and design docs |
| Vague description ("A tool for AI") | One sentence: subject + verb + object + audience |
| Too many concepts (10+) | 4–6 domain terms that require explanation; obvious names excluded |
````

**Step 3: Verify word count is under 500**

```bash
wc -w skills/populate-llmspec/SKILL.md
```

Expected: ~420 words.

**Step 4: Commit**

```bash
git add skills/populate-llmspec/SKILL.md
git commit -m "feat(skills): add populate-llmspec skill"
```

---

### Task 2: Update `codebase-indexer` skill to reference `populate-llmspec`

**Files:**
- Modify: `skills/codebase-indexer/SKILL.md` (Step 3 section, ~line 39)

**Step 1: Read the file**

Read `skills/codebase-indexer/SKILL.md` to see the current Step 3 content.

**Step 2: Replace Step 3**

Find:
```markdown
**Step 3: Review and fill gaps in `.llmspec.yaml`**

Auto-generated fields will be populated. Manually review and complete:
- `concepts` — domain terms that aren't obvious from code
- `patterns` — conventions that require judgment to identify
- `description` — one-sentence project summary
```

Replace with:
```markdown
**Step 3: Populate semantic fields**

`sensei index` fills the structural scaffold. Use the `populate-llmspec` skill to fill the semantic fields (description, concepts, patterns, docs[].covers[]) using Claude's own code understanding — higher quality than the local model.

```
invoke: populate-llmspec skill
```
```

**Step 3: Verify the file still reads cleanly**

Read `skills/codebase-indexer/SKILL.md` and confirm the flow makes sense.

**Step 4: Commit**

```bash
git add skills/codebase-indexer/SKILL.md
git commit -m "feat(skills): wire codebase-indexer → populate-llmspec for semantic fill"
```

---

### Task 3: Run populate-llmspec on this repo as a live test

This isn't a code task — it's a manual verification that the skill works end-to-end.

**Step 1: Reset llmspec.yaml to baseline**

The current `.sensei/llmspec.yaml` may have been partially populated by the coverage benchmark run. Reset the semantic fields to TODO so the skill has something to fill:

```bash
# Read .sensei/llmspec.yaml, note which fields need resetting
cat .sensei/llmspec.yaml
```

Fields to reset if they were auto-generated badly:
- `description`: reset to `"TODO: one-sentence project summary"` if it's still the placeholder
- `concepts`: set to `[]` if empty
- `patterns`: set to `[]` if empty
- `entry_points`: reset to `[{path: "src/index.ts", role: "TODO"}]` if wrong

**Step 2: Invoke the populate-llmspec skill**

Open a new Claude Code session or use `/populate-llmspec` (if skill is installed). Follow the skill protocol to fill all fields.

**Step 3: Score**

```bash
bun tasks/score-coverage.ts .sensei/llmspec.yaml
```

Expected: 70+/100. The benchmark result with local model was 32/100.

**Step 4: Commit the populated llmspec**

```bash
git add .sensei/llmspec.yaml
git commit -m "chore: populate llmspec via populate-llmspec skill"
```
