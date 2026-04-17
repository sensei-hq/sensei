---
name: MCP Workflow Tools
description: New MCP tools for workflow state, event logging, metrics, and pattern queries
date: 2026-04-17
type: design
traces:
  - blueprints/01-workflow-engine.md
  - blueprints/02-system-architecture.md
  - ideas/15-pattern-store.md
  - ideas/17-pattern-knowledge.md
---

# MCP Workflow Tools

## Overview

The sensei-mcp binary exposes daemon capabilities as MCP tools that the AI calls. Currently it has code intelligence tools (search, get_callers, etc.). It needs workflow intelligence tools for phase management, event logging, metrics, and pattern queries.

---

## New tools

### Workflow state

| Tool | Params | Returns | Called by |
|------|--------|---------|----------|
| `update_phase(phase, task?, issue?)` | phase: string, task?: string, issue?: number | `{ ok: true, state: WorkflowState }` | Phase commands |
| `get_workflow_state()` | (none) | `{ phase, task, issue, last_checkpoint, guardrails_hash }` | `/sensei:status`, `/sensei:refocus`, AI when lost |

### Event logging

| Tool | Params | Returns | Called by |
|------|--------|---------|----------|
| `log_event(type, data)` | type: string (one of 16 event types), data: JSON object | `{ ok: true, id: string }` | Commands (per instructions in command markdown) |

### Metrics

| Tool | Params | Returns | Called by |
|------|--------|---------|----------|
| `get_metrics(range?)` | range?: "7d" \| "30d" \| "all" (default: "30d") | `{ ftr, turn_count_avg, rework_rate, tool_adherence, locate_accuracy, pattern_adherence }` | `/sensei:analyze`, AI coaching |

### Pattern queries

| Tool | Params | Returns | Called by |
|------|--------|---------|----------|
| `get_patterns(type?)` | type?: string (adapter, factory, etc.) | `[{ name, type, instances, files, interface_name }]` | `/sensei:build` locate step, `/sensei:patterns` |
| `match_pattern(description)` | description: string | `[{ name, type, confidence, reference_file, invariants }]` | `/sensei:build` locate step |
| `get_pattern_for(symbol)` | symbol: string | `{ pattern_name, type, role_in_pattern }` or null | `/sensei:review` conformance check |
| `get_duplicates(file?)` | file?: string | `[{ file_a, file_b, similarity, lines }]` | `/sensei:review` |
| `get_project_conventions()` | (none) | `[{ convention, evidence_count, example_files }]` | `/sensei:build`, `/sensei:patterns` |

---

## Testing

MCP tools are Rust functions that call daemon HTTP endpoints. Test at two levels:

**Unit (Rust):** Mock the HTTP client, verify correct endpoint called with correct params.

**Integration:** Start daemon with test DB, call MCP tool, verify response matches expected data.

```rust
#[tokio::test]
async fn log_event_stores_and_returns_id() {
    let daemon = start_test_daemon().await;
    let result = log_event("phase_transition", json!({"from": "idea", "to": "analyze"})).await;
    assert!(result.ok);
    assert!(!result.id.is_empty());
    // Verify event was stored
    let events = daemon.get_events("test-project", Some("phase_transition")).await;
    assert_eq!(events.len(), 1);
}
```
