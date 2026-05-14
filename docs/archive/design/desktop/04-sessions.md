# 04 — Sessions

> Routes: `/sessions` (global), `/p/{id}/sessions` (project-scoped), `/sessions/{id}` (detail)

## Purpose

Sessions are where learning happens. Every AI coding session produces events — tool calls, file edits, corrections, phase transitions. The sessions pages surface what happened, what went right, and what went wrong.

This is the **feedback loop**. When FTR drops, the developer comes here to understand why. When a session went well, the developer can extract patterns to replicate it.

## Session List

### Global (`/sessions`)

All sessions across all repos. Paginated, sorted by recency.

### Project-scoped (`/p/{id}/sessions`)

Same view but filtered to repos in this project. This is the primary way developers look at sessions — in the context of a product.

### What Each Session Shows

| Field | Source |
|-------|--------|
| Repo name | CWD → repo mapping |
| Timestamp | Session start event |
| Duration | Start to last event |
| Outcome | `completed` / `blocked` / `partial` (from checkpoint) |
| Correction count | Number of `revision_requested` events |
| Turn count | Number of user prompts |
| FTR | Yes/No (0 corrections = FTR) |
| ACP | Which AI platform was used |

### Filters

- By repo (within project context)
- By outcome (completed / blocked / partial)
- By FTR (first-try-right only / corrections only)
- By date range

**The most valuable filter is "corrections only"** — this shows exactly which sessions had problems. This is where the developer learns what to fix.

## Session Detail (`/sessions/{id}`)

A single session's full timeline.

### Event Timeline

Chronological list of everything that happened:

```
10:15:03  session_start    CWD: /Users/dev/acme-api
10:15:03  mindset_applied  analyst
10:15:05  tool_used        search("auth handler")
10:15:08  tool_used        get_callers("handleLogin")
10:16:12  turn             "add rate limiting to login"
10:16:45  tool_used        get_patterns()
10:17:30  revision_requested  "no, use the existing RateLimiter class"  ← correction
10:18:20  tool_used        search("RateLimiter")
10:19:45  checkpoint       outcome: completed
```

Each event is expandable to show full data (tool parameters, response summaries where available).

### Correction Analysis

When a session has corrections, highlight them:

```
CORRECTIONS (1)

10:17:30  "no, use the existing RateLimiter class"
  Context: AI tried to implement custom rate limiting
  Pattern gap: No pattern for "use existing utilities before creating new ones"
  Suggestion: Extract pattern → add to rules
```

This is the core learning mechanism. Each correction is an opportunity to improve — either by adding a pattern, updating a rule, creating a persona, or adjusting which skills are active.

### Session Stats

```
Duration:    4m 42s
Turns:       6
Corrections: 1  (FTR: no)
Tools used:  search (2), get_callers (1), get_patterns (1)
Files read:  4
Files edited: 1
```

## Insights → Actions

| What you see | What to do |
|---|---|
| Session had corrections because AI didn't know about existing utility | Extract pattern: "check existing utils before creating new" → add to rules |
| Session took 15 turns for a simple change | Context was missing — check what session-start loaded, add to orientation skill |
| Session blocked because AI couldn't find the right file | Improve search patterns, check if file is indexed |
| Multiple sessions correct the same mistake | Promote to a rule or mindset question — this is a recurring gap |
| Session completed first try in a complex area | Study what made it work — which skills triggered, what context was loaded |

## What's Built

Session list page exists with dummy data. Session detail page exists with dummy data. Both need real API wiring.

## What Needs to Be Built

| Feature | Dependency |
|---------|-----------|
| Real session data | `log_event()` MCP tool + hook event capture |
| Session list API | `/api/sessions` endpoint (exists but may need enrichment) |
| Session detail API | `/api/sessions/:id/events` endpoint |
| Correction highlighting | `revision_requested` event type in hooks |
| Pattern gap suggestions | Pattern detection + event correlation in daemon |
| FTR computation | Daemon-side: sessions with 0 corrections = FTR |

**Build order:** `log_event()` → event capture in hooks → session list API → session detail API → this page. Correction analysis and pattern suggestions are Phase 2.
