---
id: response-cache
type: design
implements:
  - feature: caching
    items: [response-capture, proactive-offer, retrieval, cache-management, session-integration]
---

# Response Cache

## Overview

The Response Cache persists notable Claude outputs to Supabase (`sensei.events` or a dedicated cache table) with semantic tags so they are retrievable across sessions by query, not just by label. Entries include the original prompt, topic tags, a one-line summary, timestamp, and a TTL; `get_session_context()` surfaces up to 3 relevant entries as hints without loading full content. A maintenance pass archives entries older than 90 days that haven't been retrieved; pinned entries are exempt.

---

## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| token-efficiency | Cache hints in get_session_context() must stay under 150 tokens total for up to 3 hints |
| reliability | Cache entries must be atomic writes — no partial entries from interrupted saves |
| scalability | find_cached_response() must return in under 2s for a cache of 200 entries |
| security | Cache files must not contain credentials or secrets — agent must warn if response appears to contain them |

---

## Data Model / Storage

Cache entries are stored in Supabase (a dedicated cache table or `sensei.events` with `event_type: 'response_cache'`). The schema below represents the row/document structure:

Individual entry schema:
```json
{
  "id": "2026-03-11-abc123",
  "label": "indexing-approach-comparison",
  "summary": "Compared cocoindex vs custom indexer: cocoindex wins on speed, custom on flexibility",
  "prompt": "Which indexing approach should we use for the sensei project?",
  "content": "[full response text]",
  "tags": ["indexing", "architecture", "trade-offs"],
  "sessionId": "session-xyz",
  "timestamp": "2026-03-11T14:30:00Z",
  "ttlDays": 90,
  "pinned": false,
  "retrievalCount": 0,
  "lastRetrieved": null
}
```

---

## Algorithm / Flow

Cache maintenance:
```
Step 1: Query Supabase for all non-pinned cache entries
Step 2: For each entry where pinned === false:
  → If lastRetrieved is null and age > ttlDays: mark archived (set archived = true)
  → If lastRetrieved exists and daysSince(lastRetrieved) > ttlDays: mark archived
Step 3: On successful retrieval: increment retrievalCount, set lastRetrieved = now, extend TTL by 90 days
Step 4: Write updates to Supabase
```

Proactive offer logic:
```
After generating response:
  → If response contains: structured comparison, ranked options, design analysis, trade-off table
  → Append to response: "Want me to cache this for future sessions? (say 'cache it')"
  → Do NOT auto-cache without user confirmation
```

---

## API / Tool Contracts

```typescript
// MCP tools
cache_response(label: string, content: string, options?: { prompt?: string }): CacheEntry
// Saves response, auto-generates tags from content, returns entry id

find_cached_response(query: string): CacheEntry | null
// Semantic search over index.json summaries+tags, loads full entry only on match

list_cached_responses(): CacheSummary
// Returns: count, topics, oldest/newest dates — no full content loaded

// CLI
// sensei cache pin <id>       — mark pinned: true
// sensei cache delete <id>    — remove entry and update index
// sensei cache gc             — run maintenance pass (archive stale)
```

---

## Error Handling

```
No match (find_cached_response):  Return null, message: "No cached response found for that query."
Invalid id (pin/delete):          "Cache entry '<id>' not found."
Write failure:                    Retry once, then: "Failed to save cache entry. Check Supabase connection."
Possible secrets in content:      Warn: "Response may contain credentials — cache not saved. Review content first."
```

---

## Testing Strategy

```
Unit: src/cache/response-cache.spec.ts
  - cache_response creates entry + updates index
  - find_cached_response returns null on no match
  - TTL maintenance archives stale, skips pinned
  - retrieval extends TTL
  - list_cached_responses stays under 100 tokens for 50 entries

E2E: e2e/cache.e2e.ts
  - save → retrieve by query
  - pin → verify survives gc
  - delete → verify removed from index
```

---

## Open Questions

| Question | Status |
|----------|--------|
| | |

---

> This is a **design doc** — how it works, not what it does.
> User-facing needs belong in `docs/features/`.
> Status lives in `docs/traceability.yaml` — do not add a status table here.
