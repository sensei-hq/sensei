# Insight plane — turning observation into understanding

> Insight = M7–M13. This plane turns raw capture into analysis, metrics,
> memories, patterns, traceability and governance — the *knowledge* half of the
> loop. Status: [`../../../roadmap/02-insight.md`](../../../roadmap/02-insight.md).

| ID | Module | One-line "done" | Spec |
|---|---|---|---|
| M7 | Analyzer & Signals | Every workspace has fresh derived signals on a schedule | ✅ [`07-analyzer-and-signals/`](07-analyzer-and-signals/README.md) · 2026-10-13 |
| M8 | Metrics & Health | Every advertised metric computes, is rated, and cannot lie by omission — master-detail, with explicit gating eligibility | ✅ [`08-metrics-and-health/`](08-metrics-and-health/README.md) · 2026-10-13 |
| M9 | Memory & Knowledge | A proven lesson becomes a durable, promoted memory | ✅ [`09-memory-and-knowledge/`](09-memory-and-knowledge/README.md) · 2026-10-13 |
| M10 | Patterns & Conventions | Recurring practice (good and bad) is detected and named | ✅ [`10-patterns-and-conventions/`](10-patterns-and-conventions/README.md) · 2026-10-13 |
| M11 | Traceability & Doc Drift | A doc claim that no longer matches code is flagged with a receipt | ✅ [`11-traceability-and-doc-drift/`](11-traceability-and-doc-drift/README.md) · 2026-10-13 |
| M12 | Libraries | An unmanaged dependency is detected, wrapped and queryable | ✅ [`12-libraries/`](12-libraries/README.md) · 2026-10-13 |
| M13 | Governance & Rules | Rules resolve by scope precedence and reach the assistant | ✅ [`13-governance-and-rules/`](13-governance-and-rules/README.md) · 2026-10-13 |

## What this plane owns

The analyzer and its derived signals (M7); the metric registry, ratings and
health — including the master-detail model and per-metric gating eligibility
(2026-10-13 decision 8, [`../../../reorg.md`](../../../reorg.md) §9.6) (M8); memories
and their promotion ladder (M9); patterns and anti-patterns (M10); doc↔code
traceability and drift (M11); the library detection/wrap/query loop (M12); and
the rule/scope precedence model (M13).

## Reading order

M7 → M8 → M9 → M10 → M11 → M12 → M13. M8 is the read most teams care about; it
depends on the rest producing honest data.

## Customer

The Delivery plane (M14–M22) renders what this plane derives, and the Team plane
(T1–T6) governs and shares it. The **metrics master-detail** and the **graph
level-of-detail** are the two 2026-10-13 additions that change how this plane is
speced (reorg §9.6, §9.7).

## Legacy sources

Folded from `features/03-observatory.md` (data half), `features/05-governance.md`,
`features/metrics/{catalog,feature,architecture-metrics}`,
`spec/pipeline/{analyzer,ftr,metrics,memory,patterns,traceability,libraries,
governance,signals,narration-cache}`, `spec/governance/`,
`design/{metrics-chart-brief,governance,indexer,phases}`.