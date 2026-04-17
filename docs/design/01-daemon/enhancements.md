---
name: Daemon Enhancements
description: What needs to be built in senseid to support the workflow engine, metrics, and pattern detection
date: 2026-04-17
type: design
traces:
  - ideas/08-codebase-intelligence.md
  - ideas/07-metrics-analytics.md
  - ideas/15-pattern-store.md
  - analysis/02-graph-node-gaps.md
  - blueprints/01-workflow-engine.md
  - blueprints/02-system-architecture.md
---

# Daemon Enhancements

## Overview

The daemon (senseid) is built and running. The indexer, graph store, sessions, and HTTP API work. What's missing is the intelligence and measurement layers: rich graph nodes, event store, workflow state, metrics computation, and pattern detection.

---

## 1. Graph node enrichment

**Traces to:** [analysis/02-graph-node-gaps.md](../analysis/02-graph-node-gaps.md), [ideas/08-codebase-intelligence.md](../ideas/08-codebase-intelligence.md)

### Quick wins (data exists, just needs storage)

| # | Fix | File | Change |
|---|-----|------|--------|
| 1 | Store docstrings | `graph_writer.rs:71` | Pass `sym.docstring` instead of `None` |
| 2 | Store line_end | `graph_writer.rs` | Add `line_end` column, write from `ParsedSymbol.line_end` |
| 3 | Store is_exported | `graph_writer.rs` | Add `is_exported` column, write from `ParsedSymbol.is_exported` |

### Adapter changes (extract + store)

| # | Fix | Adapters affected | Schema change |
|---|-----|-------------------|---------------|
| 4 | Params (name + type) | All language adapters | New `params` JSON column on symbol nodes, or `TAKES_PARAM` edges |
| 5 | Return type | TS/JS (OXC), Rust, Python (type hints) | New `return_type` column on symbol nodes |
| 6 | Doc frontmatter | Doc indexer | New columns: `description`, `status`, `date`, `origin` on doc nodes |
| 7 | Implements/extends | TS/JS, Rust (traits), Python, Java | New `IMPLEMENTS` and `EXTENDS` edge types |
| 8 | Decorators/annotations | Python, Java, TS | New `decorators` JSON column on symbol nodes |

### New edge types

| # | Edge | From → To | Purpose |
|---|------|----------|---------|
| 9 | `IMPLEMENTS` | class → interface/trait | Pattern detection (adapter = implements interface) |
| 10 | `EXTENDS` | class → parent class | Class hierarchy |
| 11 | `TRACES_TO` | doc → doc (via frontmatter origin) | Traceability matrix |
| 12 | `DUPLICATES` | symbol → symbol | Duplicate detection |

### Testing

Each fix should have tests that verify the enriched node shape:

```rust
#[test]
fn symbol_stores_docstring() {
    let graph = index_fixture("fixtures/documented.rs");
    let sym = graph.find_symbol("parse_file").unwrap();
    assert!(sym.docstring.is_some());
    assert!(sym.docstring.unwrap().contains("Parse a source"));
}

#[test]
fn symbol_stores_params() {
    let graph = index_fixture("fixtures/adapter.rs");
    let sym = graph.find_symbol("parse_file").unwrap();
    assert_eq!(sym.params.len(), 2);
    assert_eq!(sym.params[0].name, "path");
}

#[test]
fn implements_edge_created() {
    let graph = index_fixture("fixtures/adapter.rs");
    let impls = graph.find_implementations("LanguageAdapter");
    assert!(impls.len() >= 1);
}
```

---

## 2. Event store

**Traces to:** [blueprints/01-workflow-engine.md](../blueprints/01-workflow-engine.md) (Data Architecture), [ideas/07-metrics-analytics.md](../ideas/07-metrics-analytics.md)

### Schema

```sql
CREATE TABLE events (
    id TEXT PRIMARY KEY,
    project TEXT NOT NULL,
    session_id TEXT,
    event_type TEXT NOT NULL,    -- phase_transition, tool_used, turn, locate, etc.
    data TEXT NOT NULL,          -- JSON payload
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_events_project ON events(project, created_at);
CREATE INDEX idx_events_type ON events(project, event_type);
CREATE INDEX idx_events_session ON events(session_id);
```

### Endpoints

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/events` | POST | Log an event (from hooks via curl, or from MCP via sensei-mcp) |
| `/api/events/:proj` | GET | List events with filters (`?type=`, `?session=`, `?limit=`, `?since=`) |

### 16 event types

See [blueprints/01-workflow-engine.md](../blueprints/01-workflow-engine.md) for full event type table.

---

## 3. Workflow state

**Traces to:** [blueprints/01-workflow-engine.md](../blueprints/01-workflow-engine.md) (section 7), [ideas/01-workflow-system.md](../ideas/01-workflow-system.md)

### Schema

```sql
CREATE TABLE workflow_state (
    project TEXT PRIMARY KEY,
    active_phase TEXT,
    active_plan TEXT,
    active_task TEXT,
    active_issue INTEGER,
    last_checkpoint TEXT,
    rules_hash TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

### Endpoints

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/state/:proj` | GET | Current workflow state |
| `/api/state/:proj` | PUT | Update workflow state (also writes `.sensei/state.yaml`) |

### State file sync

When state is updated via PUT, the daemon also writes `.sensei/state.yaml` for the project so hooks (bash, no MCP access) can read it.

---

## 4. Metrics computation

**Traces to:** [ideas/07-metrics-analytics.md](../ideas/07-metrics-analytics.md), [blueprints/01-workflow-engine.md](../blueprints/01-workflow-engine.md) (computable metrics)

### Endpoints

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/metrics/:proj` | GET | Computed metrics (`?range=7d\|30d\|all`) |
| `/api/phases/:proj` | GET | Phase transition history |

### Computed metrics (from event aggregation)

| Metric | Query |
|--------|-------|
| FTR | `SELECT COUNT(*) FILTER (WHERE NOT EXISTS revision_requested for issue) / COUNT(DISTINCT issue) FROM events WHERE type IN ('issue_completed')` |
| Turn count per task | `SELECT COUNT(*) FROM events WHERE type = 'turn' AND active_issue = ?` |
| Rework rate | `COUNT(type='rework') / COUNT(DISTINCT issue_completed)` |
| Tool adherence | `COUNT(type='tool_used' AND is_mcp=true) / COUNT(type='tool_used')` |
| Locate accuracy | Compare `locate.files_identified` with `files_modified` events for same issue |
| Phase velocity | Time between consecutive `phase_transition` events |
| Pattern adherence | `COUNT(pattern_checked AND followed=true) / COUNT(pattern_checked)` |

---

## 5. Pattern detection

**Traces to:** [ideas/08-codebase-intelligence.md](../ideas/08-codebase-intelligence.md), [ideas/15-pattern-store.md](../ideas/15-pattern-store.md), [ideas/17-pattern-knowledge.md](../ideas/17-pattern-knowledge.md)

### Phase A: Naming heuristics (Wave 1)

Add a post-indexing pass that scans symbol names for pattern indicators:

```rust
fn detect_patterns_by_name(symbols: &[Symbol]) -> Vec<PatternCandidate> {
    let suffixes = ["Adapter", "Factory", "Observer", "Builder",
                    "Strategy", "Middleware", "Handler", "Hook",
                    "Worker", "Provider", "Decorator"];
    // Group symbols by suffix, require 2+ instances to be a pattern
}
```

Store as pattern nodes in graph:
```sql
CREATE TABLE patterns (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    pattern_type TEXT,         -- adapter, factory, observer, etc.
    interface_name TEXT,       -- trait/interface they implement (if detected)
    instance_count INTEGER,
    files TEXT,                -- JSON array of file paths
    project TEXT NOT NULL
);
```

### Phase B: Structural heuristics (Wave 2+)

Requires rich nodes (implements/extends edges) from section 1, fixes 7, 9, 10.

### Testing

```rust
#[test]
fn detects_adapter_pattern_by_name() {
    let graph = index_fixture("fixtures/adapters/");
    let patterns = detect_patterns(&graph);
    let adapter = patterns.iter().find(|p| p.name == "language-adapter");
    assert!(adapter.is_some());
    assert!(adapter.unwrap().instance_count >= 2);
}
```
