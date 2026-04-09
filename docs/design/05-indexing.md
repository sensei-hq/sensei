---
id: indexer
type: design
implements:
  - feature: indexing
    items: [repo-scanner, symbol-map, llms-txt-generation, multi-modal-search, symbol-graph]
---

> **SUPERSEDED — HISTORICAL RECORD ONLY**
>
> This document describes the original regex-based extraction approach. It has been superseded twice:
> - First by `20-pipeline-adapter.md` + `40-metadata-model.md` (AST-based extraction via ts-morph)
> - Then by the current graph DB approach documented in **`ADR-001-indexing-approach.md`** (Kuzu + tree-sitter)
>
> The current indexing architecture uses Kuzu as an embedded graph DB, tree-sitter for AST parsing,
> and an L0–L5 depth model for token-efficient context loading. See ADR-001 for the full rationale,
> including what was tried and abandoned.
>
> **NFRs and framework detection tables below remain useful reference.**

---

## Alternatives Considered (Historical)

### Regex-based symbol extraction (this document)

The original approach: pattern-match export declarations per language, write to `.index/symbol-map.json`.

**Why abandoned:** Regex can extract names and signatures but cannot represent relationships. Call graphs, import graphs, and type dependency graphs are invisible to regex. Without structure, agents cannot navigate — they get a flat list with no connections.

### cocoindex (tried, abandoned)

Evaluated as a replacement for the regex extractor. Flat semantic search over code embeddings. Indexing ran; results were poor. Semantic similarity surfaces files that mention similar words, not files that are structurally connected. A query for callers of a function needs graph traversal, not cosine similarity. Tried and dropped — not in the codebase.

### MemPalace (evaluated, rejected)

Graph-augmented memory built on ChromaDB. Good recall numbers. Rejected because ChromaDB requires a server process — incompatible with the desktop app's zero-dependency requirement. The "memory palace" spatial metaphor is also designed for personal knowledge management rather than code graph navigation.

### FalkorDB (evaluated, rejected for desktop)

Strong Cypher support and TypeScript bindings. Rejected because it requires a Redis-based server process. Acceptable for a future team server product; not acceptable for a desktop app that must work out of the box.

### Current approach: Kuzu + tree-sitter

See `ADR-001-indexing-approach.md`.

---

---

# Indexing

## Overview

The indexer scans a repo and produces structured artifacts in `.index/`. It runs as an MCP tool (`reindex_repo`) and is guided by the `codebase-indexer` skill. V1 does a full rescan each time; V2 will diff against git to process only changed files.

## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| performance | Full scan of a 500-file repo must complete in under 30s |
| accuracy | Symbol extraction must correctly capture all exported functions and their signatures |
| scalability | Index artifacts must not grow unbounded — incremental runs update only changed entries |
| reliability | Index must be consistent after interrupted runs (no partial/corrupt state) |

---

## Extraction Targets

### File Map

Walk the directory tree to depth 3, excluding noise directories. Identify entry point files by name convention.

**Entry point names:** `index`, `main`, `app`, `server`, `router`, `config`, `cli`

**Excluded:** `node_modules/`, `dist/`, `.git/`, `coverage/`, `.cache/`, `build/`, `.next/`, `.nuxt/`

### Tech Stack

Read package manifests to extract language, frameworks, and tools. Ignore dev tooling (linters, formatters).

| Manifest | What to read |
|---|---|
| `package.json` | `name`, `version`, `dependencies`, `devDependencies` |
| `pyproject.toml` | `[project]` name, `[tool.poetry.dependencies]` |
| `Cargo.toml` | `[package]` name, `[dependencies]` |
| `go.mod` | module name, `require` block |
| `pom.xml` | `artifactId`, `dependencies` |

**Framework detection** (from package.json dependencies):

| Detected | Framework |
|---|---|
| `react`, `@react` | react |
| `next` | next.js |
| `vue`, `@vue` | vue |
| `svelte` | svelte |
| `express` | express |
| `fastify` | fastify |
| `hono` | hono |
| `@nestjs/core` | nestjs |
| `postgres`, `pg` | postgres |
| `prisma` | prisma |
| `drizzle-orm` | drizzle |

### Dev Shortcuts

| Source | How to extract |
|---|---|
| `package.json scripts` | All entries in `scripts` object |
| `Makefile` | Lines matching `^[a-zA-Z].*:` (target names) |
| `justfile` | Lines matching `^[a-zA-Z].*:` |
| `taskfile.yaml` | `tasks` keys |
| `scripts/` directory | All executable files |

### Symbol Map

Glob all code files, extract exported symbols, store at L0 and L1. L2 is a placeholder in V1 (requires LLM summarisation, deferred to V2).

**Code file extensions:** `.ts`, `.tsx`, `.js`, `.jsx`, `.mjs`, `.py`, `.go`, `.rs`

**Export detection regex per language:**

```
TypeScript/JavaScript:
  /^export\s+(async\s+)?(function|class|const|type|interface|enum)\s+(\w+)/

Python:
  /^def\s+(\w+)/ and /^class\s+(\w+)/  (top-level, non-underscore prefixed)

Go:
  /^func\s+([A-Z]\w*)/  (exported = capitalised)

Rust:
  /^pub\s+(fn|struct|enum|trait)\s+(\w+)/
```

**L0 format:** The matched line, trimmed, without the opening brace or body.

**L1 format:** `returnType = functionName(paramName: paramType, ...)` — extracted from signature.

**L2 format:** Placeholder `"// L2 not yet generated"` in V1. V2 will use an LLM to summarise function bodies.

### Documentation Fingerprints

Glob all documentation files, record path, size, and mtime for drift detection.

**Doc file extensions:** `.md`, `.mdx`, `.txt`, `.rst`, `.yaml`, `.yml`, `.json` (in `docs/` only)

**Excluded:** `node_modules/`, `dist/`, `.git/`, `coverage/`

---

## Output Files

### `.index/symbol-map.json`

```json
{
  "src/auth.ts": {
    "L0": [
      "export async function login(email: string, password: string): Promise<User | null>",
      "export function logout(sessionId: string): void"
    ],
    "L1": [
      "user = login(email, password)\n// returns: Promise<User | null>",
      "logout(sessionId)\n// returns: void"
    ],
    "L2": [
      "// L2 not yet generated",
      "// L2 not yet generated"
    ]
  }
}
```

### `.index/stack.md`

```markdown
# Tech Stack

## Languages
- typescript/javascript

## Frameworks
- express
- react

## Tools
- vitest
- tsx
```

### `.index/shortcuts.md`

```markdown
# Shortcuts

- **dev**: `npm run dev`
- **test**: `npm test`
- **build**: `npm run build`
- **lint**: `npm run lint`
```

### `.index/patterns.md`

Auto-generated as a placeholder. Human-reviewed and expanded after first index.

```markdown
# Patterns

<!-- Auto-generated: review and expand manually -->
<!-- Add patterns detected during code review here -->
```

### `.index/doc-index.json`

```json
{
  "README.md": { "mtime": 1741234567890, "size": 4821 },
  "docs/plans/2026-03-06-design.md": { "mtime": 1741234000000, "size": 12400 },
  "CHANGELOG.md": { "mtime": 1741200000000, "size": 3200 }
}
```

---

## Processing Order

```
1. Detect stack (fast, reads manifests only)
2. Detect shortcuts (fast, reads manifests and Makefile)
3. Build symbol map (slow, reads all code files)
4. Build doc fingerprint index (fast, stats only — no file reads)
5. Write all .index/ files in parallel
6. Write .llmspec.yaml template if missing
7. Generate llms.txt
8. Write CLAUDE.md template if missing
```

Steps 1–4 run in parallel. Step 5 writes after all are complete.

---

## Incremental Re-indexing (V2)

V1 does a full rescan on every `reindex_repo()` call. V2 will:

1. Read `git diff --name-only HEAD~1 HEAD` to find changed files
2. Re-process only changed files in the symbol map
3. Update doc fingerprints for changed doc files
4. Leave unchanged entries in `symbol-map.json` untouched

This requires the repo to be a git repo. Non-git repos fall back to full rescan.
