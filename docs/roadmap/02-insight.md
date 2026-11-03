# Roadmap — Insight plane (M7–M13)

Observation → understanding: analyzer (M7), metrics/health (M8), memory (M9),
patterns (M10), traceability (M11), libraries (M12), governance (M13). Status
legend: [`README.md`](README.md).

> **Status populated 2026-11-03.** The cells follow the readiness table in
> [`../review/2026-11-03-orientation.md`](../review/2026-11-03-orientation.md) §2 and the capability
> audit in [`../review/2026-10-27-roadmap-vs-state.md`](../review/2026-10-27-roadmap-vs-state.md) §1.2.
> The last column names what proves each cell; a status is a snapshot, so move it when the module moves.

| ID | Module | One-line "done" | Status | Evidenced by |
|---|---|---|---|---|
| M7 | Analyzer & Signals | Every workspace has fresh derived signals on a schedule | ✅ **Built** | `tasks/analyzer_scheduler.rs`; `GET /api/tasks/scheduled` lists 16 tasks; 27,674 rows in `inference.recommendations`; `/api/observatory/tool-signals` answers |
| M8 | Metrics & Health | Every advertised metric computes, is rated, and cannot lie by omission — master-detail, explicit gating eligibility, coverage-gated presentation | 🟡 **Partial** | The engine is built: registry (32 active), `repository_metrics.ddl`, `workspace_health_score.ddl`, `/api/metrics/status/summary` (157 repositories). Missing: the M-7 gating field and master-detail model, the M-8 coverage presentation (the DB half exists, no screen reads it), M-9, the confidence chain (#158–#160) and most of #156 — only `graph_confidence`, `public_surface_ratio` and `symbol_size_p95` are architecture metrics |
| M9 | Memory & Knowledge | A proven lesson becomes a durable, promoted memory | 🟡 **Partial** | `memories.ddl`, HTTP curation (`routes.rs:486–499`) and the MCP save/propose/promote tools are built; `memory_scope` still lacks the `user`, `repository` and `dojo` rungs and the verbs are not yet one **Share →** (D56, D65); there is no `report_learning` / `report_memory_use`, so the failure → lesson loop cannot close |
| M10 | Patterns & Conventions | Recurring practice (good and bad) is detected and named | ⛔ **Gap** | There is no `crates/senseid/src/patterns/`; `POST /api/patterns/{ws}/detect` only lists stored rows, and the 681 `detected_patterns` rows are behavioural (rework, corrections), not code patterns; duplication is partial (`code_duplicates.ddl`, 0 live rows) |
| M11 | Traceability & Doc Drift | A doc claim that no longer matches code is flagged with a receipt | 🟡 **Partial** | `analysis/doc_drift.rs` and `…/drift` exist but are inert: the route answers `{"items":[],"indexedDocs":0}` because there is no markdown adapter (#292); `trace_links`, `get_trace_links` and apply-fix are not built |
| M12 | Libraries | An unmanaged dependency is detected, wrapped and queryable | 🟡 **Partial** | Detection of 209 libraries (`/api/libs`), manifest ingestion, six library MCP tools, version pins and daily OSV advisories are built; only 3 of 209 libraries have any doc page, there is no `wrap_library`, and version-aware doc queries and site discovery (#342) are open |
| M13 | Governance & Rules | Rules resolve by scope precedence and reach the assistant | 🟡 **Partial** | `pg_store/governance.rs resolve_rules_raw` resolves by enforcement then scope (`general 0 < user 10 < organization 20 < workspace 60 < repository 70`, D57), 14 packs are seeded and resolved rules are pulled by `dojo/client.rs`; there is no `rule_violations` / `governance_overrides` store (G-GOV-AUDIT), the checker has 0 live runs, and team scope is open (#318) |

## Known gaps affecting this plane

| Gap | What | Priority |
|---|---|---|
| M-7 | metrics have no master-detail model and no `gating` field; gating eligibility is undefined per metric (M8) | P1 |
| M-8 | coverage-gated presentation absent — thin metrics read as strong (M8; #155 generalised) | P1 |
| M-9 | no per-metric design/coverage record tying a metric to its chart and level of detail (M8) | P1 |
| D-6 | metric doc defects (#153 `churn_rate`, #154 `time_to_useful_result` rating units) — **closed** 2026-10-19 | — |

Also in the legacy register: #156 architecture-metrics family (unblocks T3's
layer/cycle/zone-of-pain gates) is WIP in a separate backlog session (decision 8).

Full register: [`06-gaps.md`](06-gaps.md).

## Sequencing

M8 is the plane's customer; its master-detail model (reorg §9.6) must land
before T3 can promise gates. M10's pattern detector and the #156 family are the
two unbuilt feeds that starve Delivery and Team surfaces.
