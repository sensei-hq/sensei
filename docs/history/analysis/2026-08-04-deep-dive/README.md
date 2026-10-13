# Deep-dive evidence — 2026-08-04

Twelve data-grounded investigations behind the three summary docs
([metrics](../2026-08-04-metrics.md), [Observations](../2026-08-04-Observations.md),
[metrics-catalog](../2026-08-04-metrics-catalog.md)). Each section is self-contained:
a theme, the numbers, the **exact SQL** that produced them, a root cause, prioritized
recommendations, and proposed metrics.

Produced by a fan-out of parallel analyst agents querying the live daemon DB
(`sensei` on Postgres :5432) on 2026-08-04. Every headline number is reproducible with
the SQL shown inline. Spot-checks against live data are noted in the summary docs.

| # | Section | Core finding |
|---|---------|--------------|
| 01 | [Autonomy — babysitting & roadblocks](01-autonomy-babysitting-roadblocks.md) | Autonomous "crashes" are the watchdog's 3-nudge budget exhausting; `paused_on_limit` resume exists but was fired once (a smoke test) |
| 02 | [Degradation over run length](02-degradation-plan-deviation.md) | Corrected-rate is a clean dose-response on turns (0→100%); late-run degradation is real but **silent** (effort decay + hedge sediment, never logged as a correction) |
| 03 | [Amnesia — memory consolidation](03-amnesia-memory-consolidation.md) | 4,300+ inference artifacts distill to 11 memories; 8 never recalled; agent denied a Playwright+Tauri harness it used 1,003× |
| 04 | [Productivity / velocity](04-productivity-velocity.md) | Raw volume is inverted as a productivity signal — 76% of tool-calls come from the reworked 27% of sessions |
| 05 | [Quality, rework & duplication](05-quality-rework-duplication.md) | 92.3% of edits are re-edits; a live duplicate detector exists but never runs at write time |
| 06 | [Tool/content utility & registry](06-tool-content-utility-registry.md) | "70% ignored" is largely a fragment-overlap measurement artifact; tool identity is unstable; 5 disjoint content silos, no registry |
| 07 | [Insight/pattern/recommendation UX](07-insight-pattern-recommendation-ux.md) | 0/943 patterns have a description; 1/1,478 recommendations acted on; `±0%` FTR-delta is a fabricated signal |
| 08 | [Instrumentation gaps](08-instrumentation-gaps.md) | `model` 0/69, `success` 0/131,690, `tokens` 0/69 — empty columns block every downstream metric |
| 09 | [Anti-patterns screen (HTML dump distilled)](09-observations-html-distill.md) | The 406-row wall is a flat per-file churn log with no descriptions and no actions; reconciles the pasted DOM to the DB |
| 10 | [Code-graph regression](10-graph-indexing-regression.md) | Three-front collapse (community write-back 1.1%, `covers` 918× duplication, missing package/subtree levels); datable to ~2026-07-13 |
| 11 | [Regression & churn lifecycle](11-regression-churn-lifecycle.md) | Version-rescan (commit `2f6f1de9`, 2026-07-12) drives 8.6× indexing churn; 92.1% of `process_file` runs change nothing |
| 12 | [Token/cost — capture, not inference](12-token-cost-inference.md) | Exact usage is on disk in the transcripts the daemon already parses; recovered 30.5 B tokens / ~$64.8 k across 58 sessions; non-FTR costs 4.7× more |

> These files are machine-generated evidence, verified against live data. Treat the
> summary docs as the narrative and this folder as the citations.
