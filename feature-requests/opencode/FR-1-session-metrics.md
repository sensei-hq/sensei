---
target: opencode-ai/opencode
status: draft
relates-to: docs/ideas/24a-observatory-data-audit.md
---

# Session metrics: token counts, cost, and session lifecycle hooks

## Summary

Plugins and external tools building session analytics need token usage data and session lifecycle signals. Currently there's no way to know how many tokens a session consumed, what it cost, or when it ended.

## What's needed

1. **Token counts per session** — `tokens_in`, `tokens_out` exposed to hooks or via API after session completes
2. **Session end signal** — hook/event when a session closes, with aggregate metrics (turns, tokens, duration)
3. **Tool response preview** — post-tool-execution hook should include truncated response content (first 500 chars), not just tool name and exit code
4. **Quota/usage introspection** — API or CLI to query remaining usage limits

## Use case

Building a session observatory that shows developers:
- Cost per session, burn rate, quota projection
- Session drill-down with tool call inputs AND outputs
- Quality metrics (FTR, rework rate) correlated with token cost
- Which approaches are efficient vs expensive

## Why this matters for opencode

opencode's open architecture is ideal for this — the plugin system could expose richer lifecycle data than proprietary alternatives. Being first to offer session-level analytics hooks would be a differentiator.

## Proposed API

```
// Session end event
{
  "event": "session.end",
  "session_id": "...",
  "tokens_in": 24000,
  "tokens_out": 8000,
  "cost_usd": 0.35,
  "duration_seconds": 720,
  "turn_count": 8
}

// Post-tool event enrichment
{
  "event": "tool.post",
  "tool": "search",
  "response_preview": "Found 3 results...",
  "response_length": 2048
}
```
