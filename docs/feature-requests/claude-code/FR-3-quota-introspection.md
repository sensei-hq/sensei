---
target: anthropics/claude-code (or anthropic API)
status: draft
relates-to: docs/ideas/24a-observatory-data-audit.md
---

# FR-3: Quota introspection — remaining usage and limits

## Summary

There is no way for a plugin, hook, or external tool to query how much of the user's Claude usage quota remains. This blocks building quota-aware dashboards, burn rate projections, and low-quota warnings.

## Use Case

An AI-assisted developer uses Claude Code daily and wants to pace their usage. Their desktop companion shows:

- "58% quota remaining"
- "At current burn rate: ~3 days remaining"
- "Warning: < 20% quota — consider batching tasks"

Today this is impossible. The developer finds out they're over quota only when a request fails.

## Data already exists

Claude Code's `/cost` and `/usage` commands already display this data interactively. The ask is to expose the same data programmatically so plugins and hooks can consume it.

## Proposed Solution (pick any)

### Option A: Machine-readable flag on existing commands
```bash
claude /cost --json
# {"session_cost_usd": 0.35, "total_cost_usd": 4.20, "tokens_in": 142000, "tokens_out": 38000}

claude /usage --json
# {"used_pct": 42, "reset_at": "2026-05-01T00:00:00Z"}
```

### Option B: Hook payload enrichment
Include cost/usage in SessionStart or SessionEnd hook payload:
```json
{
  "quota_remaining_pct": 58,
  "quota_reset_at": "2026-05-01T00:00:00Z",
  "session_cost_usd": 0.35
}
```

### Option C: Well-known file
Write session cost data to `~/.claude/session-stats.json` that plugins can read — no API needed.

## Impact

Quota anxiety is real — developers self-censor their usage because they don't know their burn rate. Visibility would let them use Claude more confidently and pace long sessions appropriately. This is table-stakes for any usage-based service.
