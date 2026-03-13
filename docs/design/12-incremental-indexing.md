---
id: incremental-indexer
type: design
implements:
  - feature: indexing
    items: [incremental-indexing, force-full-reindex, index-summary]
---

# Incremental Indexing

## Overview

After the first full index, `reindexRepo` uses `git diff <lastIndexedCommit>..HEAD --name-only` to identify changed, added, and deleted files since the commit at index time. Only those files are re-processed. Supabase records for changed files are updated in-place. A `force` flag bypasses the diff and runs a full scan.

For non-git repos (no `.git/` directory), falls back to mtime/size fingerprints stored in `sensei.scan_state`.

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

1. Load prior `sensei.scan_state` rows for this repo
   → if no scan_state rows exist: force = true (first run)

2. Check if repo has git: stat(repoPath/.git)
   → isGit = true/false

3. Determine changed files:
   a. If force: changedFiles = ALL glob results
   b. If isGit AND doc-index has lastIndexedCommit:
      changedFiles = git diff <lastIndexedCommit>..HEAD --name-only
      deletedFiles = git diff <lastIndexedCommit>..HEAD --name-only --diff-filter=D
   c. If not git (fallback): compare mtime/size against `sensei.scan_state` fingerprints

4. For each changed/added code file:
   → re-extract exports → upsert into `sensei.symbols`

5. For each deleted file in symbol-map:
   → delete from `sensei.symbols` and `sensei.scan_state`

6. Upsert updated rows into `sensei.symbols`
7. Upsert `sensei.scan_state` rows with:
   - lastIndexedCommit: current HEAD sha (git rev-parse HEAD) — stored in `sensei.repos`
   - content_hash, mtime per file for fallback/non-git
8. Regenerate `.sensei/stack.md`, `.sensei/shortcuts.md` (always regenerate — cheap)
9. If no `.sensei/llmspec.yaml`: write template (never overwrite)
10. Regenerate `.sensei/llms.txt`
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

### `sensei.scan_state` (Supabase table)

| Column | Type | Description |
|---|---|---|
| `repo_id` | uuid | FK to `sensei.repos` |
| `file_path` | text | Relative path from repo root |
| `content_hash` | text | SHA-256 of file content |
| `mtime` | bigint | Last modified timestamp |
| `indexed_at` | timestamptz | When this file was last indexed |

`lastIndexedCommit` (git SHA of last index run) is stored in `sensei.repos`.

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
  Created: .sensei/llmspec.yaml, CLAUDE.md, .sensei/llms.txt
```

---

## Testing Strategy

```
Unit: src/tools/reindex.spec.ts (extended)
  - git: only files in git diff are re-extracted
  - git: deleted files removed from symbol-map
  - git: new files added to symbol-map
  - git: lastIndexedCommit stored in `sensei.repos` after run
  - non-git fallback: mtime/size comparison used when no .git/
  - force: all files re-extracted regardless of git diff
  - summary counts correct for each case
  - first run (no prior index): force = true implicitly
```
