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

## Proposed Solution

### Option A: CLI command
```bash
claude quota
# Output: {"used": 420000, "limit": 1000000, "remaining": 580000, "reset_at": "2026-05-01T00:00:00Z"}
```

### Option B: Hook payload enrichment
Include quota info in SessionStart hook:
```json
{
  "quota_remaining_pct": 58,
  "quota_reset_at": "2026-05-01T00:00:00Z"
}
```

### Option C: API endpoint
`GET /v1/usage` on the Anthropic API returning current period usage and limits.

## Impact

Quota anxiety is real — developers self-censor their usage because they don't know their burn rate. Visibility would let them use Claude more confidently and pace long sessions appropriately. This is table-stakes for any usage-based service.
