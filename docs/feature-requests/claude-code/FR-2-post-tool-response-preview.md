---
target: anthropics/claude-code
status: draft
relates-to: docs/ideas/24a-observatory-data-audit.md
---

# FR-2: PostToolUse hook — include response preview

## Summary

The `PostToolUse` hook fires after a tool executes but only provides the tool name and exit code. It does not include any of the tool's response content. This makes it impossible for plugins to log what a tool returned, which blocks session drill-down features.

## Use Case

Building a session analytics dashboard where a developer can click on a tool call and see what it returned. For example:

- "Turn 3: search('compute_metrics') → 3 results: [store.rs:924, routes.rs:1702, ...]"
- "Turn 5: get_lib_docs('rokkit') → 4 sections loaded"

Today I can log THAT a tool was called, but not WHAT it returned. The session timeline has blind spots.

## Proposed Solution

Add an optional `response_preview` field to the PostToolUse hook payload:

```json
{
  "tool_name": "search",
  "tool_input": {"query": "compute_metrics"},
  "exit_code": 0,
  "response_preview": "Found 3 results: store.rs:924 compute_metrics, routes.rs:1702 get_metrics, ...",
  "response_length": 2048
}
```

- `response_preview`: first N characters (e.g., 500) of the tool response, truncated
- `response_length`: full response size in characters

This keeps the hook payload small while giving plugins enough to log meaningful context.

## Alternatives

- Full response in hook: too large, would slow down hooks
- Plugin reads tool response from a temp file: fragile, no API for this
- Plugin re-executes the same tool call: wasteful, may have side effects

## Impact

Without this, session drill-down shows "search was called" but not "search found 3 results in store.rs." The developer can't understand WHY Claude made a decision without seeing what data it had.
