# Pattern Skills Design

> **Status:** approved
> **Date:** 2026-03-17

## Goal

Two skills that give agents institutional memory about how things are built in this codebase: one that discovers and documents structural patterns, and one that applies them before writing new code — with all usage tracked to Supabase.

## Architecture

Four components with single responsibilities:

**`PATTERNS.md`** (repo root) — the catalog. One entry per pattern: name, what it solves, canonical example file path, link to skill. Human-editable, committed to git. Already created empty by `reindexRepo`.

**`skills/<pattern-name>/SKILL.md`** — one skill file per pattern. Contains the implementation recipe: the shared interface shape, a canonical before/after example, step-by-step instructions, and invariants to preserve. Invocable like any other skill.

**`skills/identifying-patterns/SKILL.md`** — finds and documents patterns in two modes:
- *Guided (first run)*: user names a domain or pattern; skill reads codebase, finds 2+ usages, extracts the shape, writes the skill file, updates `PATTERNS.md`
- *Incremental (re-run)*: diffs current codebase against existing catalog, updates stale recipes, adds newly-detected patterns

**`skills/pattern-based-development/SKILL.md`** — triggered before any implementation task. Reads `PATTERNS.md`, selects most relevant pattern, loads its skill, calls `record_pattern_use()` MCP tool to start tracking, guides implementation, and links outcome + changed files back at `checkpoint()`.

---

## Supabase Schema

New table: `sensei.pattern_usages`

```sql
create table sensei.pattern_usages (
  id             uuid primary key default gen_random_uuid(),
  repo_id        uuid references sensei.repos(id) on delete cascade,
  session_id     text,  -- fk to events.session_id (nullable)
  pattern_name   text not null,
  applied_at     timestamptz default now(),
  outcome        text,   -- completed | blocked | partial (filled by checkpoint())
  files_modified text[], -- filled by checkpoint() via git diff
  ftr_score      float   -- filled by checkpoint()
);
```

Index on `(repo_id, pattern_name)` for dashboard queries.

---

## MCP Tool: `record_pattern_use`

Added to `packages/server/src/mcp-server.ts` alongside existing tools.

```typescript
record_pattern_use(pattern_name: string): string
```

- Inserts a row in `pattern_usages` with current `session_id` and `repo_id`
- Returns confirmation string
- Wrapped with `beat()` in `mcp-entry.ts` like all other tools

**`checkpoint()` extension** — after writing session outcome and FTR score, also:
1. Runs `git diff --name-only <session_start_commit>..HEAD` to get changed files
2. Updates any open `pattern_usages` rows for the same `session_id` with `outcome`, `ftr_score`, `files_modified`

---

## Skill File Structure

### `skills/identifying-patterns/SKILL.md`

```
name: identifying-patterns
description: Use when starting work in an unfamiliar area of the codebase,
before implementing something that might follow an existing structural pattern,
or when asked to document how a recurring pattern is used in this project.
Also use periodically to refresh the pattern catalog after significant code changes.
```

**Procedure:**
1. User names a domain ("adapters", "CLI commands", "dashboard routes") or requests auto-discovery
2. Search codebase for 2+ implementations of that structure using semantic search + grep
3. Read the canonical examples and extract: shared interface, required fields, invariants
4. Write `skills/<pattern-name>/SKILL.md` with:
   - What the pattern solves and when to use it
   - The interface (types/signatures the pattern always produces)
   - A canonical example (file path + key excerpts)
   - Step-by-step recipe: exact files to create/modify, what to put in each
   - Invariants: what must never change when applying this pattern
5. Append catalog entry to `PATTERNS.md`
6. In incremental mode: read existing catalog, compare each entry's canonical file against current code, flag stale entries, offer to update

### `skills/pattern-based-development/SKILL.md`

```
name: pattern-based-development
description: Use before implementing any new feature, component, module, or
integration — checks PATTERNS.md for an applicable recipe before writing new
code. Prevents re-inventing structure that already exists in this codebase.
```

**Procedure:**
1. Read `PATTERNS.md` catalog
2. Match task description to most relevant pattern(s)
3. If match found: load `skills/<pattern-name>/SKILL.md`
4. Call `record_pattern_use(pattern_name)` MCP tool
5. Present the pattern recipe to the agent before implementation starts
6. Implement following the recipe — the recipe defines file structure, interface shape, naming
7. At `checkpoint()`, outcome + files are automatically linked to the pattern usage row

---

## Generated Pattern Skill Structure

Each pattern skill file follows this template:

```markdown
---
name: <pattern-name>
description: Use when implementing <X> in this codebase — provides the
established recipe so new implementations match the existing structure.
---

# <Pattern Name> Pattern

## What It Solves
<1-2 sentences on why this pattern exists in this codebase>

## Interface
<The types/signatures every implementation of this pattern produces>

## Canonical Example
File: `<path/to/example.ts>` (lines X–Y)
<Key excerpt showing the pattern in use>

## Recipe
1. Create `<path>` with this structure: ...
2. Implement these methods/exports: ...
3. Register in `<path>`: ...

## Invariants
- <What must never be omitted>
- <What naming convention is mandatory>
- <What must be tested>
```

---

## Files Created/Modified

| File | Action |
|------|--------|
| `skills/identifying-patterns/SKILL.md` | Create |
| `skills/pattern-based-development/SKILL.md` | Create |
| `packages/server/src/mcp-server.ts` | Modify — add `record_pattern_use` tool |
| `packages/server/src/mcp-server.spec.ts` | Modify — add tests for new tool |
| Supabase migration file | Create — `pattern_usages` table |
| `PATTERNS.md` | Modify — add structure/header comment |
