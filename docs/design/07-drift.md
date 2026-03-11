---
id: drift-detector
type: design
implements:
  - feature: traceability
    items: [git-change-detection, drift-cross-reference, on-demand-reporting, pre-commit-hook, ci-integration]
---

# Doc Drift Detection

## Overview

Drift detection uses `git diff` against the last indexed commit to identify which files have changed, then cross-references those changes against a traceability matrix (`.index/traceability.json`) to flag docs whose linked code files have changed. This enables precise "code changed but doc didn't" detection without content analysis.

For non-git repos, falls back to mtime/size comparison against `doc-index.json` fingerprints.

## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| accuracy | No false positives — docs not linked to changed files must not be flagged |
| reliability | git fallback (mtime/size) must work when .git/ is absent |
| performance | Drift check on a 100-doc repo must complete in under 5s |

---

## Traceability Matrix

`.index/traceability.json` maps design/feature docs to the code files they cover:

```json
{
  "docs/design/03-mcp-server.md": ["src/index.ts", "src/tools/query.ts", "src/tools/reindex.ts"],
  "docs/design/07-drift.md": ["src/tools/drift.ts"],
  "docs/design/10-project-memory.md": ["src/tools/project-memory.ts"],
  "docs/features/01-CodebaseIndexing.md": ["src/tools/reindex.ts", "src/index-reader.ts"],
  "README.md": ["packages/sensei/package.json"]
}
```

**Population:**
- Declared in `.llmspec.yaml` under `docs[].covers[]` (manual, authoritative)
- Auto-detected by `reindexRepo` from doc content (filename mentions, symbol references) — best-effort
- Manual declarations override auto-detection

**Schema in `.llmspec.yaml`:**
```yaml
docs:
  - path: docs/design/03-mcp-server.md
    covers:
      - src/index.ts
      - src/tools/query.ts
      - src/tools/reindex.ts
```

---

## Drift Detection Algorithm

```
checkDrift(repoPath):

1. Read .index/doc-index.json → get lastIndexedCommit
   → if missing: return "No index found. Run sensei index first."

2. Check if repo has git:
   a. If git AND lastIndexedCommit present:
      changedFiles = git diff <lastIndexedCommit>..HEAD --name-only
   b. If not git (fallback):
      changedFiles = files where mtime/size differ from doc-index fingerprints

3. Read .index/traceability.json
   → if missing: skip cross-reference, report raw changed files only

4. For each (docPath, coveredFiles) in traceability:
   a. If ANY coveredFile is in changedFiles:
      → docPath is drifted if it is NOT also in changedFiles
      → label: "{docPath}: code changed (src/foo.ts) — doc may need update"

5. For each changed file that IS a doc (*.md, *.yaml outside .index/):
   → check if doc itself changed but code it covers did NOT
   → label: "{docPath}: doc changed without code change — verify alignment"

6. Return { drifted: DriftEntry[], summary: string }
```

---

## Data Structures

### `DriftEntry`

```typescript
interface DriftEntry {
  docPath: string;
  reason: 'code-changed' | 'doc-changed' | 'file-deleted' | 'raw-modified';
  changedFiles?: string[];  // code files that triggered the drift
}
```

### `DriftResult`

```typescript
interface DriftResult {
  drifted: DriftEntry[];
  summary: string;
  lastIndexedCommit?: string;
}
```

---

## Drift Report Format

No drift:
```
No drift detected. All docs aligned with code at a3f8c21.
```

With drift:
```
3 doc(s) drifted since a3f8c21:

docs/design/03-mcp-server.md: code changed — src/index.ts, src/tools/query.ts
docs/design/07-drift.md: code changed — src/tools/drift.ts
docs/features/01-CodebaseIndexing.md: code changed — src/tools/reindex.ts

Run sensei index after updating docs to clear drift.
```

---

## Pre-Commit Hook

`sensei hooks install` installs a pre-commit hook:

```bash
#!/bin/sh
# .git/hooks/pre-commit

RESULT=$(sensei drift 2>&1)

if echo "$RESULT" | grep -q "drifted"; then
  echo "⚠️  Doc drift detected. Update docs before committing:"
  echo "$RESULT"
  exit 1
fi
```

The hook runs `checkDrift` against staged changes. Blocks commit if docs are stale.

---

## Resolving Drift

1. `sensei drift` — see what drifted
2. For each drifted doc: review what code changed (use `git diff <hash>..HEAD -- <code-file>`)
3. Update the doc to reflect the code change
4. `sensei index` — update `lastIndexedCommit` and fingerprints
5. Commit code + doc changes together

---

## Fallback Behavior (non-git / no prior commit)

When `lastIndexedCommit` is absent (non-git repo or pre-index state):

- Falls back to mtime/size comparison from `doc-index.json`
- No traceability cross-reference possible in this mode
- Reports raw file modifications only: `"{path}: modified since last index"`

---

## Testing Strategy

```
Unit: src/tools/drift.spec.ts
  - git: changed code file with traceability entry → doc flagged as drifted
  - git: changed doc but code unchanged → doc-changed drift entry
  - git: file deleted → file-deleted entry
  - git: no changes since lastIndexedCommit → no drift
  - non-git fallback: mtime/size diff → raw-modified entry
  - missing traceability.json → raw changed files only, no cross-reference
  - missing doc-index.json → returns error message
```
