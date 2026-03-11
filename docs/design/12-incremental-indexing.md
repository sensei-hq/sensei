---
id: incremental-indexer
type: design
implements:
  - feature: indexing
    items: [incremental-indexing, force-full-reindex, index-summary]
---

# Incremental Indexing

## Overview

After the first full index, `reindexRepo` uses `git diff <lastIndexedCommit>..HEAD --name-only` to identify changed, added, and deleted files since the commit at index time. Only those files are re-processed. The symbol-map is patched in-place. A `force` flag bypasses the diff and runs a full scan.

For non-git repos (no `.git/` directory), falls back to mtime/size fingerprints from `doc-index.json`.

## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| performance | Incremental run touching 5 files must complete in under 3s |
| reliability | Deleted files must be fully removed from all index artifacts |
| accuracy | Incremental results must be identical to a full rescan of the same state |

---

## Algorithm

```
reindexRepo(repoPath, { force: false })

1. Load existing .index/symbol-map.json and .index/doc-index.json
   → if either missing: force = true (first run)

2. Check if repo has git: stat(repoPath/.git)
   → isGit = true/false

3. Determine changed files:
   a. If force: changedFiles = ALL glob results
   b. If isGit AND doc-index has lastIndexedCommit:
      changedFiles = git diff <lastIndexedCommit>..HEAD --name-only
      deletedFiles = git diff <lastIndexedCommit>..HEAD --name-only --diff-filter=D
   c. If not git (fallback): compare mtime/size against doc-index fingerprints

4. For each changed/added code file:
   → re-extract exports → update symbol-map

5. For each deleted file in symbol-map:
   → remove from symbol-map

6. Write updated symbol-map.json
7. Write updated doc-index.json with:
   - lastIndexedCommit: current HEAD sha (git rev-parse HEAD)
   - fingerprints: { path: { mtime, size } } for fallback/non-git
8. Write updated stack.md, shortcuts.md (always regenerate — cheap)
9. If no .llmspec.yaml: write template (never overwrite)
10. Write llms.txt (always regenerate)
11. If no CLAUDE.md: write template (never overwrite)

12. Return summary: { added, updated, removed, unchanged, forced }
```

---

## Change Detection

### Git repos (primary)

Changed files come directly from git:

```
git diff <lastIndexedCommit>..HEAD --name-only
```

This handles:
- Files modified, added, or deleted since the last index commit
- Renames (old path removed, new path added)
- Files changed by `git stash pop`, `git pull`, `git rebase`

A file is **added** if it's in `--diff-filter=A`, **deleted** if in `--diff-filter=D`, **modified** otherwise.

### Non-git repos (fallback)

A file is considered **changed** if:
```
abs(currentMtime - storedMtime) > 1000ms  OR  currentSize !== storedSize
```

The 1000ms tolerance handles filesystem rounding on some platforms.

---

## Data Structures

### `doc-index.json` (updated schema)

```json
{
  "lastIndexedCommit": "a3f8c21d4b9e0f1234567890abcdef1234567890",
  "files": {
    "src/auth.ts": { "mtime": 1741234567890, "size": 4821 },
    "docs/design/03-mcp-server.md": { "mtime": 1741234000000, "size": 12400 }
  }
}
```

`lastIndexedCommit` is absent on first run or in non-git repos.

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
  - git: only files in git diff are re-extracted
  - git: deleted files removed from symbol-map
  - git: new files added to symbol-map
  - git: lastIndexedCommit stored in doc-index.json after run
  - non-git fallback: mtime/size comparison used when no .git/
  - force: all files re-extracted regardless of git diff
  - summary counts correct for each case
  - first run (no prior index): force = true implicitly
```
