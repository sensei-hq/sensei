---
name: Execution Traces
description: Full trace per gateway call — attempt history, cost breakdown, hook events, debugging and analytics
date: 2026-04-24
status: idea
related: 01-gateway-engine.md, 11-budget-management.md
reference: /Users/Jerry/Developer/strategos/packages/core/src/types/trace.ts
---

# Execution Traces

## Problem

When something goes wrong (wrong model chosen, unexpected fallback, high cost), we need to understand exactly what happened. The execution trace captures every decision the gateway made for a single request.

## Trace structure

```rust
pub struct ExecutionTrace {
    pub id: String,
    pub request_id: String,
    pub capability: Capability,
    pub status: TraceStatus,         // Success, Failed, Partial
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration_ms: u64,
    pub candidates: Vec<CandidateInfo>,
    pub skipped: Vec<SkippedInfo>,
    pub attempts: Vec<Attempt>,
    pub estimated_cost: Option<CostEstimate>,
    pub actual_cost: Option<Cost>,
}

pub struct Attempt {
    pub sequence: u8,                // 0-indexed attempt number
    pub adapter: String,
    pub model: String,
    pub api_model_id: String,
    pub status: AttemptStatus,       // Success, Failed { reason }
    pub duration_ms: u64,
    pub tokens: Option<TokenUsage>,
    pub cost: Option<f64>,
    pub error: Option<String>,
    pub fallback_triggered: bool,
}
```

## Hook events

The engine fires three hook events that can be observed or logged:

### TaskStartEvent
```rust
pub struct TaskStartEvent {
    pub task_id: String,
    pub capability: Capability,
    pub chain: Option<String>,
    pub candidates: Vec<CandidateInfo>,
    pub skipped: Vec<SkippedInfo>,
    pub budget: Option<f64>,
    pub timestamp: DateTime<Utc>,
}
```

### AttemptEvent
```rust
pub struct AttemptEvent {
    pub task_id: String,
    pub sequence: u8,
    pub adapter: String,
    pub model: String,
    pub status: AttemptStatus,
    pub duration_ms: u64,
    pub tokens: Option<TokenUsage>,
    pub cost: Option<f64>,
    pub error: Option<String>,
    pub fallback_triggered: bool,
}
```

### TaskEndEvent
```rust
pub struct TaskEndEvent {
    pub task_id: String,
    pub status: TraceStatus,
    pub total_attempts: u8,
    pub duration_ms: u64,
    pub estimated_cost: Option<f64>,
    pub actual_cost: Option<f64>,
    pub final_adapter: Option<String>,
    pub final_model: Option<String>,
}
```

## Storage

Traces stored in `execution_traces` table (see DB issue). JSONB columns for flexible structured data.

## Use cases

| Use case | What traces tell you |
|----------|---------------------|
| Debug provider failures | Which adapter failed, error message, whether fallback worked |
| Cost analysis | Estimated vs. actual cost per call, which models cost most |
| Latency profiling | Duration per attempt, total round-trip, slowest adapter |
| Fallback frequency | How often chains fall back, which triggers fire most |
| Model effectiveness | Which models succeed most, which get skipped most |

## Open questions

| # | Question |
|---|----------|
| 1 | How long to retain traces? They grow fast. 30 days? Configurable? |
| 2 | Should traces include request/response payloads or just metadata? Payloads are useful for debugging but large and potentially sensitive. |
| 3 | Should there be a trace sampling mode for high-volume chains (embeddings)? Log every Nth trace instead of all. |
