# Doc Drift Detection

## Overview

Drift detection compares the current state of documentation files against a stored fingerprint index. The fingerprint records file path, size, and mtime. Any deviation constitutes drift. Drift is checked on demand via the `check_drift` MCP tool.

---

## Fingerprint Schema

`.index/doc-index.json`:

```json
{
  "README.md": { "mtime": 1741234567890, "size": 4821 },
  "docs/plans/2026-03-06-design.md": { "mtime": 1741234000000, "size": 12400 },
  "CHANGELOG.md": { "mtime": 1741200000000, "size": 3200 },
  "docs/features/01-CodebaseIndexing.md": { "mtime": 1741100000000, "size": 5600 }
}
```

**Fields:**
- `mtime`: milliseconds since epoch (from `fs.stat().mtimeMs`)
- `size`: file size in bytes (from `fs.stat().size`)

---

## Drift Detection Algorithm

```
checkDrift(repoPath):
  1. Read .index/doc-index.json
  2. If missing → return "No doc-index.json. Run reindex_repo first."
  3. For each (path, fingerprint) in stored index:
     a. Stat the file at REPO_PATH/path
     b. If file does not exist → add to drifted: "{path}: deleted (was in index)"
     c. If |current.mtime - stored.mtime| > 1000ms OR current.size != stored.size
        → add to drifted: "{path}: modified since last index"
  4. Return { drifted: string[], summary: string }
```

**Mtime tolerance:** 1000ms (1 second) to account for filesystem precision variation across platforms.

**Size check:** Secondary confirmation. A file with the same mtime but different size has definitely changed (handles edge cases where mtime isn't updated).

---

## What Drift Means

Drift means a file has changed since the last `reindex_repo()` call. It does NOT mean the file is wrong — it means it has changed and the index is stale. The agent decides whether the change is significant.

Typical causes:
- Code file modified without updating design docs (code ahead of docs)
- Design doc updated but public README not yet updated (design ahead of public)
- File deleted without removing references from other docs

---

## Drift Report Format

```
No drift detected. All indexed docs match current state.
```

Or:

```
3 file(s) drifted since last index:
src/auth.ts: modified since last index
docs/plans/old-design.md: deleted (was in index)
CHANGELOG.md: modified since last index
```

---

## Pre-Commit Hook

`install.sh --with-hooks` installs a pre-commit hook that runs `check_drift` and fails if drift is detected:

```bash
#!/bin/sh
# .git/hooks/pre-commit

RESULT=$(node /path/to/mcp/repo-index-server/dist/cli.js check-drift 2>&1)

if echo "$RESULT" | grep -q "drifted"; then
  echo "⚠️  Doc drift detected. Update docs before committing:"
  echo "$RESULT"
  exit 1
fi
```

The hook requires a CLI entrypoint (`dist/cli.js`) that runs `check_drift` directly without starting the full MCP server. This is a separate thin wrapper around the same `checkDrift` function.

---

## Resolving Drift

1. Call `check_drift()` to see what drifted
2. For each modified file: review what changed
3. Update the corresponding doc layers that should reflect the change
4. Call `reindex_repo()` to update fingerprints
5. Commit updated docs together with the code change

The cycle ensures code changes and doc updates are committed together.

---

## Limitations (V1)

- **Content drift not detected:** Only file modification time and size are checked, not semantic drift. A file that was edited and then reverted to the same bytes will not show as drifted.
- **New files not flagged:** Files created after the last index that are not in `doc-index.json` are not reported. V2 will add a "new doc files not indexed" check.
- **No cross-layer analysis:** V1 does not detect "this code file changed but the design doc that describes it didn't." That requires cross-referencing code and doc content — V2 with LLM analysis.
