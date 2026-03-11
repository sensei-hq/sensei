---
id: traceability-matrix
type: design
implements:
  - feature: traceability
    items: [coverage-declaration, auto-detection]
---

# Traceability Matrix

## Overview

`.index/traceability.json` maps documentation files to the source files they describe. Combined with `git diff`, this enables precise drift detection: when a code file changes, only the docs that cover it are flagged — not everything in the repo.

## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| accuracy | Manual coverage entries must never be overwritten by auto-detection |
| reliability | Matrix must remain valid (no orphan entries) after file renames |
| performance | Matrix generation must add under 5s to a full index run |

---

## Schema

### `.index/traceability.json`

```json
{
  "docs/design/03-mcp-server.md": ["src/index.ts", "src/tools/query.ts"],
  "docs/design/07-drift.md": ["src/tools/drift.ts"],
  "docs/features/01-CodebaseIndexing.md": ["src/tools/reindex.ts", "src/index-reader.ts"],
  "README.md": ["packages/sensei/package.json"]
}
```

Keys are doc paths (relative to repo root). Values are arrays of source file paths the doc covers.

### `.llmspec.yaml` declaration (authoritative)

```yaml
docs:
  - path: docs/design/03-mcp-server.md
    covers:
      - src/index.ts
      - src/tools/query.ts
      - src/tools/reindex.ts
  - path: docs/design/07-drift.md
    covers:
      - src/tools/drift.ts
```

Manual declarations in `.llmspec.yaml` are authoritative and override auto-detection.

---

## Population Strategy

### 1. Manual (`.llmspec.yaml`)

Developer declares `covers:` per doc. Most accurate. Required for high-value docs (architecture, design). `reindexRepo` reads these and writes them into `traceability.json`.

### 2. Auto-detection (best-effort)

During `reindexRepo`, for each doc file:
- Scan doc content for code file paths mentioned (`src/foo.ts`, backtick-wrapped filenames)
- Scan for symbol names that match exported symbols in the symbol-map
- Add inferred entries to `traceability.json` without overwriting manual ones

Auto-detection handles the common case where docs naturally reference the files they describe.

### 3. Fallback

If neither manual nor auto-detection produces a link, the doc is not in the traceability matrix and is not included in cross-reference drift analysis. It may still appear in raw `git diff` output.

---

## Update Lifecycle

```
reindexRepo():
  1. Read .llmspec.yaml docs[].covers → manual entries
  2. Auto-detect entries from doc content
  3. Merge: manual overrides auto
  4. Write .index/traceability.json
```

`traceability.json` is always regenerated on `reindexRepo`. Manual entries in `.llmspec.yaml` are never lost because they're the source of truth.

---

## Usage in Drift Detection

```
checkDrift():
  1. git diff <lastIndexedCommit>..HEAD --name-only → changedFiles[]
  2. Load traceability.json
  3. For each (doc, coveredFiles):
     → IF any coveredFile in changedFiles AND doc NOT in changedFiles:
        → doc is drifted: "code changed, doc may need update"
```

This gives per-doc actionable drift reports instead of "everything changed since last index."

---

## CLI Integration

```
sensei index      → regenerates traceability.json as part of full/incremental index
sensei drift      → uses traceability.json for cross-reference drift report
```

No standalone traceability command. The matrix is an internal artifact managed by `reindexRepo`.

---

## Testing Strategy

```
Unit: src/tools/reindex.spec.ts
  - manual llmspec entries written to traceability.json
  - auto-detection: doc mentioning "src/foo.ts" creates entry
  - manual entries not overwritten by auto-detection
  - traceability.json regenerated on each reindex

Unit: src/tools/drift.spec.ts
  - traceability cross-reference: changed code → linked doc flagged
  - doc without traceability entry: not included in cross-reference
  - empty traceability.json: drift falls back to raw changed files
```
