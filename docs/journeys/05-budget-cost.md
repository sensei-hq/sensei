---
name: Budget & Cost Control
description: Daily spend tracked → limit approached → alert → limit hit → graceful degradation to local models
date: 2026-04-24
status: idea
---

# Journey: Budget & Cost Control

## Scenario

A developer has a $5/day budget. They're doing heavy code review work, asking lots of chat questions routed to Claude. By afternoon, they're approaching the limit.

## Timeline

### Morning: Normal spend

```
9:00  chat(explain auth flow)         → claude-sonnet  → $0.008
9:15  chat(review PR #42)             → claude-sonnet  → $0.024
9:30  chat(suggest refactor for...)   → claude-sonnet  → $0.012
...
12:00 Daily spend: $1.87 / $5.00 (37%)
```

All background tasks (indexing, classification) use local models — $0.00.

### Afternoon: Approaching limit

```
14:00 Daily spend: $3.92 / $5.00 (78%)
14:15 chat(analyze test coverage)     → claude-sonnet  → $0.018
      Daily spend: $3.94 / $5.00

15:00 Daily spend: $4.02 / $5.00 (80%) → ALERT THRESHOLD
      Log: "⚠ Gateway budget: 80% of daily limit reached ($4.02/$5.00)"
```

### Late afternoon: Budget hit

```
16:30 Daily spend: $4.96 / $5.00
16:32 chat(explain this error trace)
  → Model selection:
    claude-sonnet-4-6: estimate $0.015 → $4.96 + $0.015 = $4.975 ≤ $5.00 → OK
  → claude-sonnet → $0.012
  → Daily spend: $4.97

16:45 chat(review this migration)
  → Model selection:
    claude-sonnet-4-6: estimate $0.035 → $4.97 + $0.035 = $5.005 > $5.00 → OVER BUDGET
    gpt-4o: estimate $0.028 → also over budget
    gemma3:27b: $0.00 → WITHIN BUDGET ← selected
  → gemma3:27b (ollama) → $0.00
  → Daily spend: $4.97 (unchanged — local model)
```

### What the user experiences

The response quality drops noticeably (local model vs Claude), but they still get an answer. No error, no interruption. The gateway status shows:

```
gateway_status:
  degradation_level: 2 (local only)
  budget: { daily_limit: 5.00, daily_spent: 4.97, daily_remaining: 0.03 }
  capabilities:
    chat: degraded (local model: gemma3:27b, reason: budget_exhausted)
    embed: available (local model: all-minilm:l6-v2)
    inference: available (local model: gemma3:27b)
```

### Next morning: Budget resets

```
00:00 Daily spend counter resets → $0.00
      Degradation level: 0 → full capability
      External providers re-enabled
09:00 chat(continue review)  → claude-sonnet → normal quality
```

## Cost breakdown view

At end of day, the developer can see:

```
Daily cost report (April 24):
  Total: $4.97

  By model:
    claude-sonnet-4-6:  $4.52 (91%) — 38 calls
    claude-haiku-4-5:   $0.23 (5%)  — 12 calls (inference chain fallback)
    gpt-4o-mini:        $0.22 (4%)  — 8 calls
    gemma3:27b:         $0.00       — 147 calls (local)
    all-minilm:l6-v2:   $0.00       — 2,340 calls (embeddings)

  By capability:
    chat:       $4.52 (38 calls)
    inference:  $0.45 (167 calls, 20 external + 147 local)
    embed:      $0.00 (2,340 calls, all local)
```

## Key design decisions

1. **Budget checks happen at selection time** — before the request is sent, not after
2. **Estimates use max_output_tokens** — worst case, so we don't accidentally exceed
3. **Local models bypass budget** — always available regardless of spend
4. **Alert at 80%** — user gets warning before quality degrades
5. **No hard block** — budget exhaustion degrades to local, never returns an error
