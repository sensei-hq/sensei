# Roadmap — Insight plane (M7–M13)

Observation → understanding: analyzer (M7), metrics/health (M8), memory (M9),
patterns (M10), traceability (M11), libraries (M12), governance (M13). Status
legend: [`README.md`](README.md).

> Per-module statuses are filled in Phase 3 from the legacy coverage map and the
> plan gap analysis (both deleted on 2026-10-21; they live in git history).
> Structure is final.

| ID | Module | One-line "done" | Status |
|---|---|---|---|
| M7 | Analyzer & Signals | Every workspace has fresh derived signals on a schedule | 🚧 Phase 3 |
| M8 | Metrics & Health | Every advertised metric computes, is rated, and cannot lie by omission — master-detail, explicit gating eligibility, coverage-gated presentation | 🚧 Phase 3 |
| M9 | Memory & Knowledge | A proven lesson becomes a durable, promoted memory | 🚧 Phase 3 |
| M10 | Patterns & Conventions | Recurring practice (good and bad) is detected and named | 🚧 Phase 3 |
| M11 | Traceability & Doc Drift | A doc claim that no longer matches code is flagged with a receipt | 🚧 Phase 3 |
| M12 | Libraries | An unmanaged dependency is detected, wrapped and queryable | 🚧 Phase 3 |
| M13 | Governance & Rules | Rules resolve by scope precedence and reach the assistant | 🚧 Phase 3 |

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