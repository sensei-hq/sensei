---
name: Developer Preferences
description: Learn personal coding style preferences from observed sessions — naming conventions, file organization, indentation styles, patterns not covered by formatters
date: 2026-04-23
status: idea
related: 07-metrics-analytics.md, 23-personas-mindsets.md, 15-pattern-store.md
---

# Developer Preferences

## Problem

Linters and formatters handle a subset of style: semicolons, bracket placement, import order. But many preferences live outside formatter scope:

- **Naming conventions** — `snake_case` for SQL columns but `camelCase` in TypeScript. Leading-comma DDL style. `_` prefix for unused variables. `I` prefix for interfaces vs no prefix.
- **File/folder naming** — `kebab-case` folders vs `PascalCase`. `.ddl` extension for SQL instead of `.sql`. `index.ts` vs named exports.
- **Structural preferences** — leading-comma SQL style with types aligned at column 27. Enum definitions on two lines. Comments using `is '...'` aligned pattern.
- **Organization patterns** — config separate from content. Views in a `view/` folder. Tests next to source vs in `__tests__/`.
- **Communication style** — terse responses, no trailing summaries. Show diffs not explanations. Don't add unsolicited docstrings.

These preferences are corrected repeatedly — the developer says "no, not like that" and the assistant adjusts, but the knowledge is lost when the session ends. The next session makes the same mistake.

## How preferences are learned

### Source 1: Observed corrections

When a developer corrects a formatting/naming/style choice, sensei records it as a preference signal.

```
Session s-2891:
  User: "one character less 27"
  → Preference signal: DDL column types align at position 27

Session s-2895:
  User: "use .ddl not .sql for these files"
  → Preference signal: SQL DDL files use .ddl extension

Session s-2900:
  User: "don't add docstrings to code you didn't change"
  → Preference signal: no unsolicited docstrings
```

After 2+ corrections on the same preference, sensei promotes it to a learned preference.

### Source 2: Codebase analysis

Scan the existing code to derive conventions:

- File naming patterns: what extensions are used, what casing
- Folder structure: how things are organized
- Indentation in non-formatter files (SQL, markdown, config)
- Comment styles, docstring patterns
- Import organization beyond what formatters handle

### Source 3: Explicit declaration

Developer explicitly states a preference:

- In project rules/guidelines: "Leading-comma SQL style"
- In persona definitions: "I prefer terse responses"
- Via a `/sensei:preference` command: "Remember: DDL types at column 27"

## Preference categories

| Category | Examples | Detection method |
|----------|---------|-----------------|
| **Naming** | snake_case columns, PascalCase components, kebab-case files | Codebase scan + correction tracking |
| **File organization** | .ddl not .sql, tests/ vs __tests__/, views in view/ folder | Codebase scan |
| **Indentation & alignment** | Column types at position 27, enum on two lines, aligned comments | Correction tracking + codebase scan |
| **Code style** | Leading-comma SQL, trailing comma JS, no semicolons in TS | Codebase scan (most covered by formatters) |
| **Communication** | Terse responses, no summaries, show diffs, no emojis | Correction tracking |
| **Workflow** | Don't add unsolicited docstrings, don't refactor surrounding code, test before claiming done | Correction tracking |

## Data model

Preferences are stored per-developer (global) and per-project (override).

### In `projects.guidelines` (project-level)

```json
{
  "guidelines": [
    {"id": "g1", "rule": "DDL column types align at position 27", "source": "learned:s-2891"},
    {"id": "g2", "rule": "Use .ddl extension for SQL DDL files, not .sql", "source": "learned:s-2895"},
    {"id": "g3", "rule": "Leading-comma style for SQL column definitions", "source": "codebase-scan"}
  ]
}
```

### In user-level config (global preferences)

```json
{
  "preferences": [
    {"id": "p1", "rule": "Terse responses, no trailing summaries", "source": "learned:s-2900"},
    {"id": "p2", "rule": "Don't add docstrings to code not being changed", "source": "learned:s-2902"},
    {"id": "p3", "rule": "Don't refactor surrounding code beyond what was asked", "source": "learned:s-2905"}
  ]
}
```

### Preference confidence

| Confidence | Criteria | Behavior |
|-----------|---------|----------|
| **High** | 3+ corrections on same topic, or explicit declaration | Include in `get_session_context()` always |
| **Medium** | 2 corrections, or strong codebase signal | Include when relevant file type/context matches |
| **Low** | 1 correction, or weak codebase signal | Store but don't inject until confirmed |

## How preferences are applied

Preferences flow into `get_session_context()` as part of the rules/guidelines section:

```
get_session_context() response:
  ...
  Rules:
  - All handlers wrap ApiError (project rule)
  - DDL column types align at position 27 (learned preference)
  - Leading-comma SQL style (codebase convention)
  Preferences:
  - Terse responses, no trailing summaries (personal)
  - Don't add unsolicited docstrings (personal)
```

This means the assistant knows the preferences from the first turn — no repeated corrections needed.

## Integration with personas (idea 23)

Preferences can be attached to personas. A "SQL development" persona carries SQL-specific preferences. A "code review" persona carries communication preferences. The persona's preferences activate when its trigger matches.

## Integration with the MOE panel (idea 25)

The reasoning panel can analyze corrections to propose new preferences:

```
MOE analysis:
  "Developer corrected column alignment 3 times in DDL files.
   Existing DDL files consistently use position 27 for types.
   Propose preference: 'DDL column types align at position 27'"
```

The developer reviews and accepts/rejects the proposed preference.

## Open questions

| # | Question |
|---|----------|
| 1 | How do we distinguish a one-time correction from a general preference? Threshold of 2 corrections? |
| 2 | Should preferences be exportable so they apply across machines? (e.g., sync via dotfiles) |
| 3 | How do we handle conflicting preferences between team members on shared projects? |
| 4 | Should preferences have a decay/TTL? A preference from 6 months ago may no longer apply. |
| 5 | How granular should codebase-derived conventions be? Every pattern in every file, or just strong signals? |
