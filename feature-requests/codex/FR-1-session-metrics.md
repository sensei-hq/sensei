---
target: openai/codex
status: draft
relates-to: docs/ideas/24a-observatory-data-audit.md
---

# Session metrics: token counts, cost, and session lifecycle events

## Summary

Codex executes tasks in sandboxed environments but doesn't expose session-level metrics (token usage, cost, duration) to external tools or plugins. This blocks building analytics dashboards for AI-assisted development workflows.

## What's needed

1. **Token counts per task** — `tokens_in`, `tokens_out` available after task completion (via webhook, API, or CLI output)
2. **Task completion event** — structured signal when a Codex task finishes, including metrics
3. **Tool/action response data** — when Codex uses tools (file read, bash, etc.), expose what the tool returned (truncated preview)
4. **Usage introspection** — API to query remaining quota/credits

## Use case

Building a development session observatory:
- "This Codex task used 48k tokens and cost $0.52"
- "Tasks using MCP tools had 22% higher success rate"
- "At current usage, ~5 days of quota remaining"

## Proposed: Structured task result

```json
{
  "task_id": "...",
  "status": "completed",
  "tokens": {"input": 48000, "output": 12000},
  "cost_usd": 0.52,
  "duration_seconds": 45,
  "actions": [
    {"type": "file_read", "path": "src/parser.rs", "preview": "..."},
    {"type": "bash", "command": "cargo test", "exit_code": 0}
  ],
  "files_modified": ["src/parser.rs"],
  "patches_applied": 1
}
```

## Why this matters

Codex's cloud execution model is ideal for structured output — the sandbox already tracks actions internally. Exposing this data lets developers build cost-aware, quality-tracking workflows on top of Codex.
