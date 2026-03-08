---
name: populate-llmspec
description: Use when .sensei/llmspec.yaml has TODO placeholders or empty fields, when docs[].covers[] entries are missing or incomplete, or when concepts and patterns fields are unpopulated after sensei index runs.
---

# Populate LLMSpec

## Overview

`sensei index` generates the structural scaffold of `.sensei/llmspec.yaml` but leaves semantic fields as TODO. This skill fills those fields using MCP tools and targeted file reads — no local model required.

## Protocol

**Step 1: Orient**

```
call: get_llmspec()
```

Note every field that is TODO, empty, or missing — including `docs[]`.

**Step 2: Fill `description`**

Read `README.md` at L1. Write a one-sentence summary and update the field.

**Step 3: Fill `entry_points`**

Read `package.json` for `bin` entries. In a monorepo, check workspace package manifests if root has no `bin`. Call `list_exports(path)` for each entry point found. Write each with a `role` phrase.

**Step 4: Fill `concepts`**

Read files in `docs/design/` at L0. Extract domain terms not obvious from code names. Add as `concepts` list items.

**Step 5: Fill `patterns`**

```
call: find_pattern("convention OR pattern OR rule")
```

Cross-reference `.sensei/symbol-map.json` for confirmed symbol names. Add patterns with exact symbol paths.

**Step 6: Fill `docs[].covers[]`**

For each doc listed in `docs[]`:

1. Read the doc at L1.
2. Ask: which source files does this doc *primarily document* — meaning it explains their API, algorithm, or design?
3. Use exact paths from `.sensei/symbol-map.json`.
4. Apply these rules:
   - Only files the doc directly documents, not tangential mentions
   - Exclude test/spec files, config files, stub files
   - Leave `docs/plans/` and `docs/templates/` entries with empty `covers: []` — do not populate them
   - Max ~5 files per doc unless it is a broad overview doc

**Step 7: Write the updated file**

Write the updated content back to `.sensei/llmspec.yaml`.

**Step 8: Score (if gold standard exists)**

```bash
bun tasks/score-coverage.ts .sensei/llmspec.yaml
```

If score < 0.8, revisit Step 6 for low-scoring doc entries.

**Step 9: Commit**

```bash
git add .sensei/llmspec.yaml
git commit -m "chore: populate llmspec semantic fields"
```

## Quick Reference

| Field | Primary source | Effort |
|---|---|---|
| `description` | README.md L1 | Low |
| `entry_points[].role` | package.json bin + list_exports | Low |
| `api_surface` | list_exports per entry point | Medium |
| `concepts` | docs/design/ L0 scan | Medium |
| `patterns` | find_pattern + symbol-map | Medium |
| `docs[].covers[]` | Read each doc at L1 | High |

Note: `stack`, `shortcuts`, `project`, and `version` are auto-populated by `sensei index` — no manual attention needed.

## Common Mistakes

| Mistake | Fix |
|---|---|
| Listing files the doc only mentions | Only include files the doc explains in depth |
| Including test or stub files in covers | Exclude `*.spec.ts`, `__stubs__/`, `*.test.ts` |
| Populating docs/plans/ covers entries | Skip plans — they are artifacts |
| Using paths not in symbol-map | Verify each path exists in `.sensei/symbol-map.json` |
| Broad overview doc capped at 5 files | Allow more for READMEs and architecture docs |
