---
name: Semantic Search Layer
description: Replace grep-first codebase search with sensei's hybrid Postgres search (full-text + semantic + structural) as the primary search layer for ACPs
date: 2026-04-24
status: idea
related: 08-codebase-intelligence.md, 25-playground-and-insights.md, 14-context-delivery.md
---

# Semantic Search Layer

## Problem

When an ACP (Claude, Copilot, etc.) needs to find something in a codebase, the workflow is:

```
User question → ACP thinks → grep → read → read → read → ACP thinks → answer
                               ↑ fast        ↑ expensive (tokens, round-trips)
```

Grep is keyword-only. It finds "playground" in a filename but can't answer "where did we discuss exploring MCP tools interactively?" The ACP compensates by reading multiple files and reasoning over them — spending tokens and time on work that a smarter search layer could eliminate.

Sensei already has a structural understanding of the codebase (call graphs, patterns, communities) and is building full-text and semantic search in Postgres. The question is: **can sensei become the primary search layer, not just an optional tool?**

## Approach: Hybrid search with grep fallback

### Three search modes in one query

Postgres gives us all three in one engine:

| Mode | Implementation | Beats grep when... |
|------|---------------|--------------------|
| **Full-text** | tsvector + GIN indexes | stemming, ranking, phrase proximity ("MCP playground" as concept) |
| **Semantic** | pgvector embeddings | natural language queries without exact keywords |
| **Structural** | code graph tables (callers, callees, patterns, communities) | relationship queries ("what calls this", "what module owns this") |

A single search query combines all three signals and returns one ranked result list. Grep can only do keyword matching, poorly.

### Grep as fallback layer

Sensei wraps grep internally as a fourth search mode. When the index doesn't cover a file (new, unindexed, unsupported type), grep catches it:

```
sensei search("where is the playground ideation?")
    │
    ├─ full-text: tsvector match on "playground" → 3 results, ranked
    ├─ semantic: embedding similarity on "playground ideation" → 2 results
    ├─ structural: docs linked to journey-07, idea-25 → 1 result
    ├─ grep fallback: rg "playground" → 12 results (unranked)
    │
    ▼
  Merge + rank → top result: docs/ideas/25-playground-and-insights.md
                  with context: "MCP tool exploration and insights engine"
```

The fallback guarantees sensei never returns fewer results than grep alone. Trust is preserved — sensei can only be better, not worse.

### Confidence-based response

Each result carries a confidence signal:

| Confidence | Meaning | What ACP sees |
|------------|---------|---------------|
| **High** | Multiple search modes agree | Direct answer with source |
| **Medium** | One mode matched strongly | Answer with "also consider..." alternatives |
| **Low** | Only grep fallback matched | Results returned but flagged as keyword-only |
| **None** | Nothing found anywhere | Explicit "not found" — no silent empty results |

## Routing: Transparent to the ACP

### Option A: Hook-based interception (Claude Code)

```
PreToolUse hook on Grep/Glob
  → extract the search intent
  → route to sensei search()
  → if confident result: return it, ACP skips grep
  → if low confidence: let grep proceed normally
```

Claude thinks it's grepping. Sensei answers first when it can.

### Option B: MCP-first instructions (any ACP)

CLAUDE.md / agent instructions tell the ACP to call `sensei.search()` before falling back to its own tools. Less transparent but works with any coordinator.

### Option C: Combined

Hooks for Claude Code (transparent), instructions for other ACPs (explicit). Sensei's search() handles both paths identically.

## Index freshness

This only works if the index is trustworthy. Key requirements:

| Requirement | Approach |
|-------------|----------|
| **Near-realtime** | File watcher (notify/fsevents) → incremental reindex on save |
| **Fast enough** | Target < 100ms for hybrid search (grep fallback adds ~5ms) |
| **Complete enough** | Index all project files; grep fallback covers gaps |
| **Honest** | If index is stale or partial, say so in confidence signal |

Grep never lies — it returns too much but doesn't miss things. Sensei must match this guarantee, which the grep fallback layer provides.

## What this enables

1. **Fewer tokens per query** — ACP gets the right file on first try instead of reading 3-5 candidates
2. **Natural language code questions** — "where do we handle auth token refresh?" works without the user knowing file names
3. **Relationship-aware search** — "what calls this function" is instant, not grep → read → trace
4. **Cross-content search** — finds relevant docs, ideas, journeys alongside code
5. **Search quality metrics** — every search is logged, enabling the insights engine (idea 25) to measure and improve search effectiveness

## Sequence

1. **PgStore CRUD** — sessions, projects, tags, workflow state (in progress now)
2. **Content indexing pipeline** — file watcher → parse → store with tsvector + pgvector embeddings
3. **Grep fallback integration** — wrap ripgrep as internal search mode with result merging
4. **Hybrid search endpoint** — `search()` MCP tool becomes the real thing
5. **Hook-based routing** — PreToolUse intercept for transparent ACP integration
6. **Search quality tracking** — log queries, results, whether ACP used them (feeds into idea 25)

## Open questions

| # | Question |
|---|----------|
| 1 | What embedding model for semantic search? Local (via Ollama) keeps it offline but quality varies. |
| 2 | How granular should indexing be — file-level, function-level, paragraph-level? Finer grain = better results but larger index. |
| 3 | Should the grep fallback be configurable (off for speed, on for completeness)? |
| 4 | How do we handle large repos where full reindex is slow? Incremental-only after initial index? |
| 5 | Can search quality metrics bootstrap the system — learn which results the ACP actually uses and upweight similar content? |
| 6 | How does this interact with context delivery (idea 14) — search finds the file, context delivery decides how much of it to send? |
