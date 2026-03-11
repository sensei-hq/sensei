---
id: pattern-store
type: design
implements:
  - feature: patterns
    items: [pattern-detection-code, pattern-detection-docs, pattern-templates, pattern-capture, pattern-search, pattern-export]
---

# Pattern Store

## Overview

The Pattern Store scans code and design docs during indexing to detect recurring structural patterns, persists them to `.sensei/patterns.md`, and exposes search and capture tools so agents can retrieve relevant patterns by semantic query or record new ones at the moment of recognition. Each pattern entry links back to its source (code paths or design doc), supports deduplication on update, and survives re-indexing without overwriting manually captured entries.

---

## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| accuracy | Auto-detected patterns must have 2+ usages before being recorded; manual patterns are never overwritten by auto-detection |
| token-efficiency | `list_patterns()` response must be under 200 tokens for up to 20 patterns |
| reliability | Manual patterns persist across reindex runs |
| scalability | Pattern detection must complete within the normal index run time for repos up to 500 files |

---

## Data Model / File Layout

`.sensei/patterns.md` file layout. Each entry:

```markdown
## adapter-per-package
**Description:** Each package exposes a typed adapter. Consumer imports adapter, not internal implementation.
**When to use:** When you have multiple interchangeable backends and want to isolate consumers from implementation details.
**When NOT to use:** When there is only one implementation and no extensibility planned.
**Sources:** src/indexer/adapters/, src/cache/adapters/
**Doc reference:** docs/design/05-indexer.md
**Captured:** 2026-03-11 (auto | manual)
```

**Pattern Templates:** Built-in patterns shipped with sensei covering: tree-vs-flat, adapter, strategy, command, observer. Each template has: name, when-to-use, when-not-to-use, code example.

---

## Algorithm / Flow

Detection during index:

```
Step 1: Scan src/ — group files by structural similarity (exports pattern, file layout)
Step 2: If count(group) >= 2 → candidate pattern
Step 3: For each candidate: check if already in patterns.md
  → If exists + auto: update sources list
  → If exists + manual: leave unchanged
  → If not exists: append new entry
Step 4: Scan docs/design/ for adapter/strategy/pattern descriptions
  → Extract named patterns with doc references
  → Same deduplication logic
```

---

## API / Tool Contracts

```typescript
// MCP tools
add_pattern(name: string, description: string, options?: { sources?: string[], docRef?: string }): void
// Saves pattern to .sensei/patterns.md, deduplicates on name match

find_pattern(query: string): PatternMatch | null
// Semantic search over patterns.md, returns: name, description, when-to-use, sources, confidence

list_patterns(): PatternSummary[]
// Returns: name + one-line description for all patterns, under 200 tokens

get_pattern(name: string): PatternDetail | null
// Returns full entry including when-to-use, when-not-to-use, example paths

// CLI
// sensei pattern export <name> [--update]
// Creates or updates skills/<name>/SKILL.md from the pattern entry
```

---

## Error Handling

```
Pattern not found (find_pattern):  Return null, do not error
Pattern not found (get_pattern):   "Pattern 'X' not found. Run reindex_repo() or add_pattern() to capture it."
Duplicate on add_pattern:          Merge (update description + sources), not duplicate
Export conflict (--update absent): "skills/<name>/SKILL.md already exists. Use --update to overwrite."
```

---

## Testing Strategy

```
Unit: src/patterns/pattern-store.spec.ts
  - auto-detection requires 2+ usages
  - manual pattern survives reindex
  - add_pattern deduplicates on name
  - list_patterns response under 200 tokens for 20 patterns

E2E: e2e/patterns.e2e.ts
  - full index → detect pattern → retrieve with find_pattern
  - export to skill → verify SKILL.md content
```

---

## Open Questions

| Question | Status |
|----------|--------|
| | |

---

> This is a **design doc** — how it works, not what it does.
> User-facing needs belong in `docs/features/`.
> Status lives in `docs/traceability.yaml` — do not add a status table here.
