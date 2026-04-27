---
name: Library Intelligence
description: Index third-party and internal libraries — docs, APIs, patterns — so the AI uses current docs instead of hallucinating
date: 2026-04-17
status: idea
sources: features/05-library-intelligence.md, design/16-local-model-indexer.md, design/06-compression.md
---

# Library Intelligence

## Problem

The AI hallucinates API usage for libraries outside its training data. When docs exist (llms.txt, README, API reference), the AI should fetch and use them instead of guessing. Current implementation has foundations but the pipeline isn't fully wired.

## Current state

- llms.txt generation: exists but HTTP endpoint to serve it not implemented
- Library doc fetching via MCP: `get_lib_docs` tool exists but has arity bug (gap-analysis C1)
- Custom library indexing from source: planned, not wired into pipeline
- External doc registry: planned, not built
- Library skill generation: planned, not built
- Unknown lib detection skill exists in plugin (`identify-unknown-libs`)

## What this idea covers

- **Library doc pipeline**: detect deps from manifests → check if docs are indexed → fetch/index if missing → serve to AI
- **Custom/internal library indexing**: index internal shared libraries from source, not just external
- **External doc registry**: community-maintained registry of library doc URLs, with caching and versioning
- **Library skill generation**: auto-generate focused skills from indexed library docs (e.g., "how to use kavach auth")
- **llms.txt serving**: HTTP endpoint for `GET /llms.txt` so other tools can consume it

## Open questions

- How do we handle library version differences? Pin to project's lockfile version?
- Registry: centralized service or distributed (git-based)?
- Should library docs be indexed into the graph alongside code symbols?
