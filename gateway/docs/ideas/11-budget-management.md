---
name: Budget Management
description: Cost estimation, daily/monthly limits, spend tracking — local models are free, external providers are metered
date: 2026-04-24
status: idea
related: 03-model-selection.md, 09-fallback-degradation.md, 12-execution-traces.md
reference: /Users/Jerry/Developer/strategos/packages/gateway/src/services/budget-filter.ts
---

# Budget Management

## Problem

External LLM providers charge per token. Without budget controls:
- A runaway indexing job could rack up hundreds of dollars overnight
- A misconfigured chain defaulting to external providers burns money on tasks local models handle fine
- Users have no visibility into what inference costs them

## Budget model

```rust
pub struct Budget {
    pub daily_limit: f64,        // e.g. $5.00/day
    pub monthly_limit: f64,      // e.g. $50/month
    pub alert_threshold: f32,    // e.g. 0.8 = alert at 80% usage
}
```

Local models (Ollama) always have zero cost. Budget only constrains external providers.

## Cost estimation

Before executing, the gateway estimates cost to decide if a model is affordable:

```rust
pub fn estimate_cost(
    pricing: &ModelPricing,
    input_tokens: u32,
    max_output_tokens: u32,
) -> CostEstimate {
    CostEstimate {
        estimated: (input_tokens as f64 * pricing.input_per_1k / 1000.0)
                 + (max_output_tokens as f64 * pricing.output_per_1k / 1000.0),
        minimum:   (input_tokens as f64 * pricing.input_per_1k / 1000.0),
        maximum:   (input_tokens as f64 * pricing.input_per_1k / 1000.0)
                 + (max_output_tokens as f64 * pricing.output_per_1k / 1000.0),
        currency: "USD".into(),
        model: pricing.model.clone(),
    }
}
```

The estimate uses `max_output_tokens` for the upper bound — actual cost may be lower.

## Model pricing

```rust
pub struct ModelPricing {
    pub input_per_1k: f64,       // cost per 1K input tokens
    pub output_per_1k: f64,      // cost per 1K output tokens
    pub per_request: Option<f64>, // flat fee per call (some providers)
    pub per_image: Option<f64>,   // for vision/generation models
}
```

Example pricing (approximate):

| Model | Input/1K | Output/1K |
|-------|---------|----------|
| claude-haiku-4-5 | $0.0008 | $0.004 |
| claude-sonnet-4-6 | $0.003 | $0.015 |
| gpt-4o-mini | $0.00015 | $0.0006 |
| gpt-4o | $0.0025 | $0.01 |
| gemma3:27b (Ollama) | $0.00 | $0.00 |

## Budget filtering

During model selection, over-budget models are skipped:

```rust
pub fn filter_by_budget(
    candidates: &[SelectedModel],
    budget_remaining: f64,
) -> BudgetFilterResult {
    let (affordable, over_budget): (Vec<_>, Vec<_>) = candidates
        .iter()
        .partition(|c| c.cost_estimate.map_or(true, |e| e.estimated <= budget_remaining));

    BudgetFilterResult { affordable, over_budget }
}
```

Models with no pricing (local) always pass the filter.

## Spend tracking

Every inference call records its cost in `inference_calls.cost_usd`. Budget queries:

```sql
-- Today's spend
SELECT COALESCE(SUM(cost_usd), 0) FROM inference_calls
WHERE recorded_at >= CURRENT_DATE;

-- This month's spend
SELECT COALESCE(SUM(cost_usd), 0) FROM inference_calls
WHERE recorded_at >= date_trunc('month', CURRENT_DATE);

-- Spend by model
SELECT model, SUM(cost_usd) FROM inference_calls
WHERE recorded_at >= CURRENT_DATE
GROUP BY model ORDER BY SUM(cost_usd) DESC;
```

## In-memory caching

Querying the DB on every call is expensive. Cache spend in-memory:

```rust
pub struct SpendCache {
    daily_spend: AtomicF64,          // refreshed periodically
    monthly_spend: AtomicF64,
    last_refresh: AtomicInstant,
    refresh_interval: Duration,      // e.g. 60 seconds
}
```

After each call, increment the atomic counter. Periodically sync with DB (handles multi-process scenarios).

## Enforcement rules

| Condition | Action |
|-----------|--------|
| `daily_spend < daily_limit` | Normal routing |
| `daily_spend >= daily_limit * alert_threshold` | Log warning, continue |
| `daily_spend >= daily_limit` | Skip external providers, local only |
| `monthly_spend >= monthly_limit` | Skip external providers, local only |
| No local models available + budget exceeded | Noop response |

**Never block. Always degrade.**

## User visibility

Budget status exposed via:
- `gateway_status` MCP tool: `{ budget: { daily_limit, daily_spent, monthly_limit, monthly_spent } }`
- Health endpoint: `GET /api/health` includes budget summary
- Desktop observatory: visual budget gauge (future)

## Open questions

| # | Question |
|---|----------|
| 1 | Should budget be per-project or global? Per-project is more granular but harder to manage. |
| 2 | Should the gateway support "burst" — allow exceeding daily limit for high-priority requests (user-initiated chat)? |
| 3 | Should actual cost (from provider response headers) replace estimated cost when available? Some providers return exact usage. |
| 4 | Should there be budget alerts (notification when approaching limit) or just silent degradation? |
