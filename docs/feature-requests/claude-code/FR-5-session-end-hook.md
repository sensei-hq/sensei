---
target: anthropics/claude-code
status: draft
relates-to: docs/ideas/24a-observatory-data-audit.md
---

# FR-5: SessionEnd hook with aggregate metrics

## Summary

There is a `SessionStart` hook but no corresponding `SessionEnd` hook. When a session closes (user exits, conversation ends, context resets), plugins have no signal and no summary data. This blocks session-level analytics.

## Use Case

A session analytics plugin needs to:
- Record when a session ended
- Capture aggregate metrics (total turns, total tokens, outcome)
- Compute session duration
- Trigger post-session processing (e.g., update dashboard, run quality checks)

Today the plugin knows when a session starts but never knows when it ends. Sessions in the database have `started_at` but `completed_at` is only populated if the user explicitly calls `checkpoint()`.

## Proposed Solution

A new hook event `SessionEnd` that fires when:
- The user exits Claude Code (`/exit`, Ctrl+C, close window)
- The conversation is cleared (`/clear`)
- Context is compacted (session effectively resets)

Payload:
```json
{
  "session_id": "...",
  "duration_seconds": 720,
  "turn_count": 12,
  "tool_calls": 8,
  "tokens_in": 48000,
  "tokens_out": 12000,
  "exit_reason": "user_exit"
}
```

## Why not use existing hooks?

- `PreCompact` fires before compaction but doesn't mean session end
- `Stop` hook fires on `/exit` but provides no session metrics
- There's no hook for window close or Ctrl+C

## Impact

Combined with FR-1 (token counts), this gives plugins a clean, single event to capture everything they need for session analytics. Without it, plugins have to reconstruct session boundaries from event timestamps — fragile and incomplete.
