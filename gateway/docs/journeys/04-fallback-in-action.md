---
name: Fallback in Action
description: Provider goes down → chain walks to next candidate → circuit breaker engages → user never notices
date: 2026-04-24
status: idea
---

# Journey: Fallback in Action

## Scenario

Anthropic's API has a brief outage. The developer is mid-session, asking questions about their codebase. They should never know the outage happened.

## Timeline

### T+0: Normal operation

```
chat("explain the auth middleware")
  → chat_chain: claude-sonnet-4-6 (anthropic) ← selected
  → Response in 1.2s, cost $0.008
  → User sees: clean explanation
```

### T+5min: Anthropic starts failing

```
chat("what calls validate_token?")
  → chat_chain: claude-sonnet-4-6 (anthropic) ← selected
  → POST api.anthropic.com → 503 Service Unavailable
  → circuit_breaker.record_failure("anthropic:claude-sonnet-4-6")
    state: Closed { failure_count: 1 }
  → should_trigger_fallback(ProviderError, triggers)? YES
  → FALLBACK → gpt-4o (openai) ← next in chain
  → Response in 0.9s, cost $0.007
  → User sees: clean explanation (from GPT-4o, they don't know)
```

Trace shows: attempt 1 (anthropic, failed), attempt 2 (openai, success).

### T+6min: Failures accumulate

```
4 more requests hit Anthropic first, all fail and fallback to OpenAI.
circuit_breaker state: Closed { failure_count: 5 }
  → threshold reached (5) → TRANSITION TO OPEN
  → next_retry: T+11min (5 minute timeout)
```

### T+7min: Circuit breaker active

```
chat("how does the retry logic work?")
  → Model selection checks circuit breaker:
    anthropic:claude-sonnet-4-6 → OPEN → SKIP (reason: CircuitBreakerOpen)
  → Next candidate: gpt-4o (openai) ← selected directly
  → Response in 0.8s
  → User sees: instant response (no wasted attempt on Anthropic)
```

Now requests skip Anthropic entirely — no 503 delay, no wasted round-trip.

### T+11min: Recovery probe

```
Circuit breaker timeout expires.
Next request:
  → anthropic:claude-sonnet-4-6 → OPEN, but next_retry reached
    → TRANSITION TO HALF_OPEN
    → Allow test request
  → POST api.anthropic.com → 200 OK!
  → circuit_breaker.record_success("anthropic:claude-sonnet-4-6")
    state: HalfOpen { success_count: 1 }
```

### T+12min: Recovery confirmed

```
2 more successful requests to Anthropic.
  → HalfOpen { success_count: 3 } → threshold reached
  → TRANSITION TO CLOSED
  → Anthropic fully restored in routing
```

### T+13min: Back to normal

```
chat("review this PR diff")
  → claude-sonnet-4-6 (anthropic) ← selected, circuit breaker Closed
  → Response in 1.1s
  → Normal operation restored
```

## What the user experienced

| Time | What user saw | What actually happened |
|------|-------------|----------------------|
| T+0 | Normal response | Claude (Anthropic) |
| T+5 | Normal response (maybe 200ms slower) | Claude failed → GPT-4o caught it |
| T+6-10 | Normal responses | GPT-4o direct (Anthropic skipped) |
| T+11-12 | Normal responses | Recovery probes to Anthropic |
| T+13+ | Normal response | Back to Claude |

**Total disruption the user noticed: zero.** Slightly different response style for ~7 minutes (GPT-4o vs Claude), but no errors, no delays, no configuration changes.

## What the traces show

```
inference_calls for the period:
  T+5:  anthropic/claude-sonnet (failed), openai/gpt-4o (success) — fallback
  T+6:  anthropic/claude-sonnet (failed), openai/gpt-4o (success) — fallback ×4
  T+7:  openai/gpt-4o (success) — anthropic skipped (circuit open)
  T+11: anthropic/claude-sonnet (success) — half-open probe
  T+13: anthropic/claude-sonnet (success) — normal
```

The insights engine can later analyze: "Anthropic had a 7-minute outage on April 24. All requests were served via OpenAI fallback. No user-facing errors."
