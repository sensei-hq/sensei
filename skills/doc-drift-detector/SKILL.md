---
name: doc-drift-detector
description: Use when you suspect design docs are out of sync with code, before committing changes that affect documented APIs or flows, or when setting up a repo to catch drift automatically.
---

# Doc Drift Detector

## Overview

Design docs, code, and public docs drift apart silently. This skill uses fingerprinting (mtime + size) stored in `.index/doc-index.json` to detect when docs haven't been updated to match code changes. Drift is caught at commit time via a pre-commit hook.

## When to Use

- Before merging a PR that changes documented behaviour
- After a refactor to verify all three doc layers (design, code, public) stay in sync
- To set up automated drift detection on a new repo

## Check Drift

```
call: check_drift()
```

Returns a report of files that have changed since the last index, grouped by doc layer:
- **design** — `docs/`, `ADRs/`, `docs/plans/`
- **code** — source files
- **public** — `README.md`, `docs/guides/`, `CHANGELOG.md`

## Drift Report Format

```
Drift detected in 3 files:

[design]  docs/design/03-mcp-server.md  — modified 2 days ago, not re-indexed
[code]    src/tools/query.ts             — modified 5 hours ago
[public]  README.md                      — last indexed 14 days ago

Run: sensei index   to re-index and clear drift
```

## Pre-Commit Hook

Install with:
```bash
sensei hooks install --drift
```

This writes `.git/hooks/pre-commit`:
```sh
#!/bin/sh
sensei drift --fail-on-drift
exit $?
```

The hook fails the commit if drift is detected, forcing the developer to either update the docs or explicitly re-index.

## Resolving Drift

1. **Update the docs** — edit the drifted doc to reflect the code change, then `sensei index`
2. **Re-index only** — if the change is minor and docs are still accurate: `sensei index`
3. **Suppress** — if docs intentionally lag (e.g. WIP): `sensei drift --ignore <file>`

## Fingerprint Storage

Fingerprints are stored in `.index/doc-index.json`:
```json
{
  "docs/design/03-mcp-server.md": { "mtime": 1710000000, "size": 4200 },
  "src/tools/query.ts":           { "mtime": 1710001000, "size": 1800 }
}
```

A file is "drifted" if its current mtime or size differs from the stored fingerprint.
