---
name: Task Queue
description: Scan-to-connect indexing pipeline with ordered task processing
type: design
---

# Task Queue

The task queue processes indexing work in a defined order: scan → repo → folder → file → resolve → connect.

## Pipeline stages

1. **Scan** — discover repos in watched folders
2. **Repo** — extract metadata (git info, package files)
3. **Folder** — walk directory tree, classify files
4. **File** — parse symbols, extract imports
5. **Resolve** — match imports to symbols
6. **Connect** — create graph edges
