# Incremental Indexing

## Overview

After the first full index, `reindexRepo` compares current file mtimes and sizes against the stored `doc-index.json` fingerprints. Only changed, added, or deleted files are re-processed. The symbol-map is patched in-place. A `force` flag bypasses the diff and runs a full scan.

---

## Algorithm

```
reindexRepo(repoPath, { force: false })

1. Load existing .index/symbol-map.json and .index/doc-index.json
   → if either missing: force = true (first run)

2. Glob all code files (*.ts, *.tsx, *.js, *.py, etc.)
3. Glob all doc files (*.md, *.yaml, etc.)

4. For each code file:
   a. Stat current mtime + size
   b. Compare against stored doc-index fingerprint
   c. If changed/added OR force: re-extract exports → update symbol-map
   d. If unchanged: keep existing symbol-map entry

5. For each file in symbol-map NOT in current glob result:
   → file was deleted: remove from symbol-map

6. Write updated symbol-map.json
7. Write updated doc-index.json (all current fingerprints)
8. Write updated stack.md, shortcuts.md (always regenerate — cheap)
9. If no .llmspec.yaml: write template (never overwrite)
10. Write llms.txt (always regenerate)
11. If no CLAUDE.md: write template (never overwrite)

12. Return summary: { added, updated, removed, unchanged }
```

---

## Change Detection

A file is considered **changed** if:
```
abs(currentMtime - storedMtime) > 1000ms  OR  currentSize !== storedSize
```

The 1000ms tolerance handles filesystem rounding on some platforms.

A file is considered **added** if it appears in the current glob but not in the stored fingerprints.

A file is considered **deleted** if it appears in the stored symbol-map or doc-index but not in the current glob.

---

## Data Structures

### `IndexSummary` (return type)

```typescript
interface IndexSummary {
  added: number;
  updated: number;
  removed: number;
  unchanged: number;
  forced: boolean;
}
```

### Updated `reindexRepo` signature

```typescript
async function reindexRepo(
  repoPath: string,
  options?: { force?: boolean }
): Promise<IndexSummary>
```

---

## CLI Integration

```
sensei index             → reindexRepo(cwd)         — incremental by default
sensei index --force     → reindexRepo(cwd, { force: true })
```

Output format:
```
Indexing... done
  3 files updated, 1 added, 1 removed, 42 unchanged
```

On first run (full scan):
```
Indexing... done (full scan)
  47 files indexed
  Created: .llmspec.yaml, CLAUDE.md, llms.txt, .index/
```

---

## Testing Strategy

```
Unit: src/tools/reindex.spec.ts (extended)
  - incremental: only changed files re-extracted
  - incremental: deleted files removed from symbol-map
  - incremental: new files added to symbol-map
  - force: all files re-extracted regardless of fingerprints
  - summary counts correct for each case
  - first run (no prior index): force = true implicitly
```
