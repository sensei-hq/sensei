---
name: Metrics
type: feature
kind: functional
module: project
status: consolidated
updated: 2026-08-08
consolidates:
  - ../../analysis/2026-07-31-ftr-friction-and-constitution-distillation.md
  - ../../analysis/2026-08-04-metrics.md
  - ../../analysis/2026-08-04-metrics-catalog.md
  - ../../analysis/2026-08-05-ai-delivery-metrics-research.md
reuses:
  - ../../spec/pipeline/ftr.md
  - ../../spec/pipeline/impact.md
  - ../../spec/pipeline/signals.md
tags: [metrics, ftr, rework, trends, retro, insights, project]
---

# Metrics

> The measurement half of sensei's retrospective loop: turn the sessions + code
> graph sensei already captures into a small set of **measured, first-party,
> paired** numbers — and the per-session retro that turns those numbers into
> "what happened, and what to do about it."
>
> Every metric answers three questions on its face: **what does this tell me ·
> what can I do about it · how do I make it better.** A number that can't answer
> all three is noise, and we don't ship noise.

This doc consolidates four analyses that never became a feature:
[FTR friction](../../analysis/2026-07-31-ftr-friction-and-constitution-distillation.md),
[Metrics & agentic behavior](../../analysis/2026-08-04-metrics.md),
[the Metrics catalog](../../analysis/2026-08-04-metrics-catalog.md), and
[the AI-delivery-metrics research](../../analysis/2026-08-05-ai-delivery-metrics-research.md).
The **data model + computation** is the design step that follows this
consolidation — see [`spec/pipeline/metrics.md`](../../spec/pipeline/metrics.md).

## What it is

sensei measures whether the developer + assistant pairing is getting *better*,
on the developer's own delivery data — not against borrowed benchmarks. It is a
**catalog** of named metrics (each self-describing: what it tells you, how to
read it, which direction is good), a **value store** at the grain each is
computed (per session, per day) that **rolls up** into day/week/month/quarter
trends, and a **per-session retro** that narrates the numbers.

It is not a productivity dashboard. Raw output is, per the research, often
*negative* value — 8 of the top-10 sessions by volume are the worst
(corrected / `ftr=false`). This feature measures **outcome**, not output.

## Why this metric set — the philosophy (consolidated)

We surveyed the metrics the industry circulates for "AI's impact on delivery"
and **adopt none of the benchmark numbers.** The
[research](../../analysis/2026-08-05-ai-delivery-metrics-research.md) is blunt:
the METR 2025 RCT found AI made experienced developers **19% slower while they
believed it made them ~20% faster** (a ~39-pt perception gap); vendor
"throughput/stability" figures swing 10× between sources; "AI rework 35–40% /
2.5× human" is uncited while the one real dataset (GitClear) shows ~1.5×. A
number you can't reproduce on your own data can't be improved or defended.

So we keep a set defined by four properties — each is a rule this feature obeys:

1. **Measured, not believed.** Every metric is instrumented from real sessions /
   git / the code graph. The perception trap doesn't apply.
2. **First-party, not borrowed.** *Our* numbers on *our* delivery — reproducible,
   improvable, defensible to a client.
3. **Paired by construction.** Every velocity metric is defined *with* its
   quality/rework counterpart (the DORA / DX Core 4 rule), so we can't tell only
   the flattering half. **FTR and rework ratio are always shown together.**
4. **Outcome-anchored, not output-anchored.** We measure whether the work was
   *right* (FTR), never LOC / edit-count / suggestion-acceptance (high acceptance
   of low-quality code is *worse*, not better).

What we take from the credible research is the **methodologies, not the
headlines**: GitClear's churn definition (reverted-or-rewritten within ~2 weeks),
Faros/DORA's rework-rate-as-5th-key, DX Core 4's paired scorecard — with our own
measured values, not the marketed multipliers.

| Our metric | Replaces the circulating claim | Why ours is better |
|---|---|---|
| **FTR** — shipped without a correction | "% productive" perception surveys | outcome not belief; per-session (≈68% today) |
| **Rework ratio** + cost-of-rework | "AI rework 35–40% / 2.5× human" | our own sessions (0.76 of tool-calls from corrected sessions; non-FTR costs 4.7×) |
| **Churn / turnover** (reverted <14–30d) | GitClear "4× clones" | GitClear's *definition*, our real value — no inflated multiplier |
| **Duplication ratio** | "declining DRY" narratives | measured from the code graph at write-time |
| **Change-failure / regression** | "change-failures +30%" | DORA-native, paired with throughput |
| **Cost / tokens per session/feature** | (absent from the set) | the DX "cost" dimension — makes rework quantifiable in money |
| **Perception-vs-reality delta** | the whole survey genre | bakes in the METR lesson: track the gap, don't trust self-report |

**Positioning:** the differentiator is not a better benchmark to cite — it's the
ability to **prove first-time-right and low rework on real delivery data.**

## North-star: FTR, and the friction it exposes (consolidated)

**FTR — first-turn resolution** — is the north-star: the fraction of sessions
whose first attempt landed without a correction. Every metric is judged by
whether it makes FTR go up or exposes why it went down.

The [friction analysis](../../analysis/2026-07-31-ftr-friction-and-constitution-distillation.md)
found the load-bearing patterns:

- **Friction concentrates in big, multi-surface repos** — cross-cutting changes.
  (The sensei monorepo itself was the hotspot at FTR 0.31 vs 1.00 on small repos.)
  → metrics must be **per-module**, not just per-project, to locate friction.
- **Six recurring miss-patterns** account for most corrections, each a *class*
  worth governing: P1 re-deriving a settled decision · P2 trusting a masked
  signal instead of the real result · P3 scoping from the title not the resolved
  design · P4 blast-radius blindness on a shared type/schema · P5 claiming "done"
  before checking live data · P6 shipping source, not the running artifact.
  (P2 and P5 are the FTR-killers — a false "green" and a false "done" both ship
  defects.)
- **The measure → distill → govern loop must close.** The whole point of
  measuring is that friction becomes governance: corrections cluster into
  candidate rules/skills, a rule **earns its place by measured correction
  reduction** ("measure, then keep what helps"), and FTR rises. The metric layer
  is the evidence that lets a rule prove it helped — or be dropped.

So metrics are not a scoreboard; they are the **feedback signal of sensei's own
improvement loop**: friction → governance → higher FTR, measured.

## The measure → distill → govern loop (owned by this feature)

This loop lives **here, in the metrics feature** — governance *consumes* it, but
the loop's evidence and its closing belong to metrics. The stages:

1. **Measure.** Compute FTR + rework + the per-module friction map (rework
   density, reopen rate, repeat-mistake) — this feature.
2. **Distill.** Cluster the corrections behind low FTR into candidate rules/skills
   (the six miss-patterns are the seed classes; P2 "verify the outcome" and P5
   "done means verified against live data" are the FTR-killers).
3. **Govern.** Promote a candidate to a `learned` rule — but a rule **earns its
   place only by measured correction reduction** ("measure, then keep what
   helps"); one that doesn't move FTR is dropped.

The evidence that lets a rule prove (or fail) its keep is the metric layer:
**memory-promotion rate** shows whether distillation is firing at all (≈0 today —
the loop is stalled: 1 cluster, 0 learned rules from 21 corrections), and the FTR
**impact annotation** shows whether an adopted rule actually reduced corrections.
Metrics own the loop's start (measure) and its verdict (did it help); governance
owns the rule store. Keeping the loop here is why `memory_promotion`,
`repeat_mistake`, and `regression_rate` are metrics, not governance internals.

## The catalog — families and coverage (consolidated)

Metrics group into families, each computed at a **cadence** (session · daily ·
project · run · tool · account). The
[catalog](../../analysis/2026-08-04-metrics-catalog.md) is the buildable source
of truth (definition · formula · source column · coverage-today), governed by the
principle **"measure, then keep what helps": you can't keep what you can't
compute, so honest coverage comes first.**

| Family | What it answers | v1 (computable today) | Blocked-on |
|---|---|---|---|
| **Outcome** | was the work right? | FTR, rework ratio, run-completion | regression rate → drift upsert |
| **Cost** | what did it cost? | (cost-of-rework recoverable) | tokens/price → transcript-usage capture + price table |
| **Velocity** | how much *right* work? | throughput (sessions/day) | complexity-weighted delta → `nodes.degree` + graph-delta |
| **Quality** | is the code healthy? | churn concentration, rework density, duplication | quality-delta → in-loop scanner (qlty); per-module coverage/complexity |
| **Autonomy** | how much babysitting? | interruption rate | resume-success → run signals |
| **Knowledge** | is it learning? | memory-promotion rate (≈0 today — *that's the signal*) | recall-hit → `memory_loads.session_id` |
| **Tool** | is the tooling used? | unused-tool count | outcome-utility → event join |

> **Every metric is expanded in [`catalog.md`](./catalog.md)** — its calculation
> (formula), source (`table.column` + live coverage), cadence, direction, how to
> read it, and **how it's represented** (chip / trend / Pareto / heatmap / gauge).
> That per-metric detail is the seed for `sensei.metrics`.

The active metrics also roll into one **derived project health score** (0–100,
each metric normalized by its direction and combined by weight) — a single
"is this project healthy?" number that trends like any other metric. See the
[spec](../../spec/pipeline/metrics.md#project_health--the-derived-health-score).

**Data-integrity rules (sensei's never-fabricate rule):** a metric with no data is **catalogued-but-empty** and shows "not yet
measured" — never a defaulted `0`. An estimate is tagged `estimated`, never
confused with a `measured` truth. Money-facing metrics (cost) **fail closed** on
a missing price — an error/absent state, never a defaulted rate.

## The actionable framing — every metric, three answers

The catalog's `purpose` / `how_to_read` / `direction` are the contract every
surface renders against — not decoration. Worked example (FTR):

- **What does this tell me?** How often the assistant's first attempt landed
  without a correction — the health of the pairing.
- **What can I do about it?** If it dips, open the sessions that broke it; the
  retro names the module and the cause (schema change, sparse instructions).
- **How do I make it better?** Adopt the memory/pattern/rule the retro surfaced;
  the impact annotation on the trend confirms whether the fix moved the number.

Applied to the graph itself (ties to the forthcoming visualization feature): a
**scattered** graph says "organize by module / layer"; a tightly-clustered one
says the structure is coherent. The *shape* is a quality signal; the metrics are
its *health*. Every visual states what it tells you, what to do, how to improve.

## The session retro — numbers into narrative

Numbers are a fact; "FTR dipped because three sessions reopened the auth module
after a schema change" is an insight. Each analyzed session gets a short retro —
**what went well · what went wrong · a candidate insight** — that is
**model-authored** over the session's deterministic facts (ftr flag, correction
turns, rework files, churn, tokens) and **honest-null on model failure** (never a
templated retro; the facts still stand). The retro feeds the memory + pattern
pipelines — it is where friction becomes a candidate rule.

## Where it fits

- **Module:** `project` (project-level content · visual · actions), collated with
  `overview`, `impact`, `atlas`, `patterns`, `traceability`. Also rolls up to the
  account/solution scope.
- **Consumes:** `activity.sessions` (ftr/outcome/model), `turns`,
  `task_executions` (churn), `nodes` + `get_duplicates` (duplication), `memories`.
- **Feeds:** the project overview/impact chips, a metrics dashboard (follow-on
  visual spec), and the memory/pattern/governance loop (via retro insights).
- **Reuses, does not duplicate:** [`spec/pipeline/ftr.md`](../../spec/pipeline/ftr.md),
  [`spec/pipeline/impact.md`](../../spec/pipeline/impact.md),
  [`spec/pipeline/signals.md`](../../spec/pipeline/signals.md).

## The shape of the data (design follows this doc)

Three concerns, deliberately kept feature-level here — the concrete schema +
computation is the design step in [`spec/pipeline/metrics.md`](../../spec/pipeline/metrics.md):

1. **A metric catalog** — one self-describing row per metric (key, family, type,
   unit, direction, purpose, how-to-read), seeded from the catalog doc.
2. **A value store** — the value at the grain it's computed (per session, per
   day), scoped to a project and optionally a module (folder), with a
   `measured`/`estimated` source. Coarser grains (week/month/quarter) are
   **rollup views**, not stored rows; ratios re-derive from numerator/denominator,
   never averaged-of-averages.
3. **A session retro** — per-session went-well / went-wrong / insight, model-
   authored, honest-null.

Decisions already taken (carried into the design): the value store is
**generalized** (project-scoped, nullable module + session, a grain column) so it
serves delivery *and* per-module quality metrics from one place; coarser grains
are **views**; the retro is **model-authored, honest-null**.

## Acceptance — Gherkin scenarios (feature-level)

```gherkin
Feature: Measured, first-party, paired metrics with an honest retro
  So that a developer can prove whether the pairing is improving and act on it,
  sensei measures outcome (not output) on their own delivery data, pairs every
  velocity number with its quality counterpart, and never fabricates a value.

  Background:
    Given the sensei project has at least one enriched session
    And the metrics catalog is seeded from the metrics catalog analysis

  Scenario: Every metric is self-describing and directional
    When I read the metrics catalog
    Then each metric has a family, type, direction, purpose, and how_to_read
    And FTR is direction "higher_better" and rework ratio is "lower_better"

  Scenario: FTR is never shown without its companion
    When a surface renders FTR for a scope and window
    Then the rework ratio for the same scope and window is available beside it

  Scenario: Outcome over output
    Given a session that changed many lines but was corrected
    When metrics are computed
    Then it does not rank as high-velocity
    And it is reflected in rework ratio, not celebrated as throughput

  Scenario: A missing metric is honest, not fabricated
    Given a metric whose source column is empty
    When metrics are computed
    Then no value is written for it
    And the surface shows "not yet measured", never a defaulted zero

  Scenario: Cost fails closed on a missing price
    Given token usage exists but no price for the model
    When cost-of-rework is computed
    Then no defaulted cost is written
    And an explicit "price unavailable" state is recorded

  Scenario: Trends roll up without averaging averages
    Given daily FTR values across three weeks
    When I request the weekly trend
    Then each week re-derives FTR from that week's numerator and denominator

  Scenario: Friction is locatable per module
    Given corrections concentrated in one module of a multi-surface repo
    When per-module metrics are computed
    Then that module's rework density is higher than the project average

  Scenario: The retro turns numbers into an insight, honestly
    Given an enriched session
    When the retro is generated
    Then it lists what went well and what went wrong with backing metric facts
    And on model failure the retro is null, never a templated summary

  Scenario: A rule earns its place by measured reduction
    Given a learned rule adopted after a correction cluster
    When FTR is measured over the sessions after adoption
    Then the impact annotation shows whether FTR rose
    And a rule that does not reduce corrections is a candidate to drop
```

## Open questions (for the design step)

- Per-module code-quality metrics (coverage/complexity/LCOM) — scanner in-loop at
  session end, or a scheduled snapshot? (See the quality-metrics blueprint.)
- Retro on every session or only enriched/non-trivial ones (cost)?
- Account-grain metrics (autonomy/limit-reset) — same store with null
  `project_id`, or a separate account-scoped table?

*(Resolved: the measure → distill → govern loop is owned by this feature — see the
section above — with governance as the consumer of its learned rules.)*

## Related

- [[spec/pipeline/metrics]] (design) · [[spec/pipeline/ftr]] ·
  [[spec/pipeline/impact]] · [[spec/pipeline/signals]] ·
  [[blueprints/2026-08-06-project-quality-metrics]] · [[features/04-project]] ·
  [[analysis/2026-08-05-ai-delivery-metrics-research]] ·
  [[analysis/2026-07-31-ftr-friction-and-constitution-distillation]]
