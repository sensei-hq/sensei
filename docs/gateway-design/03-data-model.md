---
name: Data Model
description: Database tables for gateway persistence — inference calls, execution traces, budget tracking
date: 2026-04-24
status: design
---

# Data Model

## Tables

The gateway needs two tables for persistence. These live in the consumer's database (e.g. sensei's PostgreSQL), not in the gateway crate itself.

### inference_calls

Every gateway call is recorded for budget tracking and analytics.

```sql
CREATE TABLE inference_calls (
    id                      uuid            PRIMARY KEY DEFAULT gen_random_uuid()
  , session_id              uuid            -- FK managed by consumer
  , project_id              uuid            -- FK managed by consumer
  , capability              text            NOT NULL
  , chain_id                text
  , adapter                 text            NOT NULL
  , model                   text            NOT NULL
  , api_model_id            text
  , input_tokens            integer
  , output_tokens           integer
  , cost_usd                numeric(10,6)   DEFAULT 0
  , duration_ms             integer         NOT NULL
  , status                  text            NOT NULL
  , error_type              text
  , fallback_sequence       smallint        DEFAULT 0
  , request_metadata        jsonb
  , response_metadata       jsonb
  , recorded_at             timestamptz     NOT NULL DEFAULT now()
);

CREATE INDEX idx_inference_calls_project
    ON inference_calls(project_id);
CREATE INDEX idx_inference_calls_session
    ON inference_calls(session_id);
CREATE INDEX idx_inference_calls_recorded
    ON inference_calls(recorded_at);
CREATE INDEX idx_inference_calls_model
    ON inference_calls(model);
CREATE INDEX idx_inference_calls_capability
    ON inference_calls(capability);
```

### execution_traces

Full execution trace for debugging and analytics. One per gateway call.

```sql
CREATE TABLE execution_traces (
    id                      uuid            PRIMARY KEY DEFAULT gen_random_uuid()
  , inference_call_id       uuid            REFERENCES inference_calls(id)
  , request_id              text            NOT NULL
  , capability              text            NOT NULL
  , status                  text            NOT NULL
  , duration_ms             integer         NOT NULL
  , candidates              jsonb           NOT NULL
  , skipped                 jsonb
  , attempts                jsonb           NOT NULL
  , estimated_cost          jsonb
  , actual_cost             jsonb
  , created_at              timestamptz     NOT NULL DEFAULT now()
);

CREATE INDEX idx_exec_traces_call
    ON execution_traces(inference_call_id);
CREATE INDEX idx_exec_traces_status
    ON execution_traces(status);
CREATE INDEX idx_exec_traces_created
    ON execution_traces(created_at);
```

## Budget queries

```sql
-- Daily spend
SELECT COALESCE(SUM(cost_usd), 0)
  FROM inference_calls
 WHERE recorded_at >= CURRENT_DATE;

-- Monthly spend
SELECT COALESCE(SUM(cost_usd), 0)
  FROM inference_calls
 WHERE recorded_at >= date_trunc('month', CURRENT_DATE);

-- Spend by model (today)
SELECT model
     , COUNT(*)           AS calls
     , SUM(cost_usd)      AS total_cost
     , AVG(duration_ms)   AS avg_duration
  FROM inference_calls
 WHERE recorded_at >= CURRENT_DATE
 GROUP BY model
 ORDER BY total_cost DESC;

-- Spend by capability (today)
SELECT capability
     , COUNT(*)           AS calls
     , SUM(cost_usd)      AS total_cost
  FROM inference_calls
 WHERE recorded_at >= CURRENT_DATE
 GROUP BY capability
 ORDER BY total_cost DESC;

-- Failed calls (for debugging)
SELECT adapter, model, error_type, COUNT(*)
  FROM inference_calls
 WHERE status = 'failed'
   AND recorded_at >= CURRENT_DATE
 GROUP BY adapter, model, error_type
 ORDER BY COUNT(*) DESC;
```

## Storage trait

The gateway crate defines a trait for persistence; consumers implement it:

```rust
#[async_trait]
pub trait GatewayStore: Send + Sync {
    async fn insert_inference_call(&self, call: &InferenceCall) -> Result<Uuid>;
    async fn insert_execution_trace(&self, trace: &ExecutionTrace) -> Result<Uuid>;
    async fn get_spend_since(&self, since: DateTime<Utc>) -> Result<f64>;
    async fn get_spend_by_model_since(&self, since: DateTime<Utc>) -> Result<Vec<(String, f64)>>;
}
```

Sensei implements this on `PgStore`. Other consumers can implement for SQLite, in-memory, or skip persistence entirely (noop store).

## Note on FK constraints

`session_id` and `project_id` on `inference_calls` are not FK-constrained at the gateway level — the gateway doesn't know about sessions or projects. The consumer (sensei) adds constraints in their migration.
