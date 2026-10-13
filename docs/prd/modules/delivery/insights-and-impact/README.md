# Insights & Impact · M19

**Plane:** delivery · **Status:** [roadmap/03-delivery.md](../../../../roadmap/03-delivery.md) ·
**Serves:** team developer ([persona 2](../../../03-personas.md#2--team-developer--the-inheritor)),
maintainer ([persona 3](../../../03-personas.md#3--maintainer--the-curator)),
manager / delivery lead ([persona 6](../../../03-personas.md#6--manager--delivery-lead--the-roll-up))

## Purpose

Insights & Impact turns the signal stream into a **verdict on the assistant's
effectiveness** and a **narration the user can trust**. It measures whether sensei's
advice moved the project's FTR up or down, composes a honest headline/body/revision
for each case, and surfaces the evidence (the memory or recommendation that drove
it) so the user can verify. The job is **no fabricated consensus, no fabricated
zero** — if there's no signal, say nothing; if the delta is within ±0.05, say
neutral; if positive, reinforce with the source memory; if negative, challenge with
a concrete revision. The user feels this as *I can see whether sensei helped, and
why.*

> **Status honesty.** The verdict engine, the narration cache, and the model
> insight ranking are built. **The `insights` table, `applied_recommendations`,
> `impact_regressions`, and `GET /api/impact` are unbuilt** — they appear in the
> legacy screen spec but have no DDL and no code. The per-project impact endpoint
> (`GET /api/projects/{id}/impact`) exists and is tested; the observatory-level
> `/api/insights` also exists.

## In scope / out of scope

- In: `MeasureVerdicts` (recomputes FTR delta vs `baseline_ftr`, bands at ±0.05);
  the verdict variants `used` / `partial` / `ignored` → `positive` / `neutral` /
  `negative` with idempotent memory creation; the narration cache (key =
  sha256(kind + canonical json)); model insight ranking (winner / no winner logic);
  per-project impact endpoint; observatory tool-insights endpoint.
- Out: the `insights` table (does not exist in DDL); `applied_recommendations`
  table; `impact_regressions` table; `GET /api/impact` (unbuilt); the planner
  baseline/ratchet (C-1 → M20).

## Surfaces

- **API** — `GET /api/projects/{id}/impact` · `/impact-verdicts` (list/create) ·
  `/impact-verdicts/{id}` (decide) · `GET /api/insights` (observatory tool-insights).
- **Daemon** — `verdicts.rs` (`MeasureVerdicts`, `compose_headline`,
  `compose_body`, `suggested_revision`); `model_insight.rs`; `tasks/handlers/verdicts.rs`;
  `api/handlers/project_detail.rs` (impact endpoints).
- **App** — project Impact pane; observatory Insights lane.
- **Data** — `sensei.project_ftr_metrics` (view `project_metrics`), `sensei.memories`
  (promoted by verdict), `activity.sessions`, `narration_cache` (key = sha256).

## Data invariants

- **Lead.** FTR delta bands at **±0.05** (5 percentage points). `MeasureVerdicts`
  reads `project_metrics.ftr_14d` and compares to `baseline_ftr`; > +0.05 →
  positive, < −0.05 → negative, else neutral.
- **Lead.** Positive → reinforce; negative → challenge. A positive verdict
  idempotently promotes a `reinforce` memory; a negative verdict idempotently
  promotes a `challenge` memory with a concrete `suggested_revision`. Neutral
  creates no memory.
- **Lead.** No fabricated zero. When no models are captured, the models list is
  empty — never a fabricated vote or consensus panel.
- **Lead.** Narration cache key is `sha256(kind + canonical_json(inputs))`.
- **Lead.** Model insight ignores low-sample models; winner needs a clear lead and
  two qualifying models.
- **Lead.** Wire camelCase: `baselineFtr`, `currentFtr`, `deltaPp`, `verdict`,
  `headline`, `body`, `revision`, `sourceMemoryId`, `models[]`.

## Signals shown

| Element | Meaning | Worked example |
|---|---|---|
| Verdict badge | positive / neutral / negative | `positive · +7pp` |
| Headline | one-line lift or regression | `FTR rose to 72% (+7pp) — sensei__search` |
| Body | the delta and the source | `Baseline 65% → current 72%. Source: memory #m-12 (reinforce)` |
| Revision | concrete suggestion for negative | `Prefer sensei__index for symbol lookup` |
| Source memory | the promoted memory id | `m-12` |
| Models list | real models, or empty | `[{name: "sonnet-4", ftr: 0.78}, …]` or `[]` |
| Trend markers | applied events on the FTR chart | markers at `applied_at` |

## Done gate

- `MeasureVerdicts` computes the delta, bands correctly, and promotes the right
  memory type idempotently.
- `compose_headline` / `compose_body` / `suggested_revision` produce stable,
  honest copy for all three verdicts.
- The narration cache hits on identical inputs; a changed input is a miss.
- Model insight ranks correctly and declines when the lead is too small or samples
  are too few.
- The project impact endpoint returns the verdict, the trend markers, and the
  source memory link.

```bash
curl -s http://localhost:7744/api/projects/$ID/impact | jq '.verdict, .headline, .models|length'
```

## Wrong gate

- **The verdict is `positive`/`negative` when `|delta| ≤ 0.05`** — band regression.
- **A positive verdict creates no memory, or a negative verdict omits the
  revision** — memory promotion regressed.
- **`models[]` contains a fabricated entry when no models were captured** — the
  empty-list invariant regressed.
- **The narration cache serves a stale entry after the inputs change.**
- **A negative verdict fabricates a "consensus panel" instead of a single
  concrete revision.**
- **The impact chart shows a flat zero when FTR is null** — null must render as
  null, not zero.

## Scenarios

- [`scenarios/insights.feature`](scenarios/insights.feature) — the verdict engine,
  narration and model insight (pinned).
- [`scenarios/impact.feature`](scenarios/impact.feature) — the impact endpoint,
  the unbuilt tables/endpoints (`NO TEST` / `# gap: C-8`).

## Open gaps

- [C-8](../../../../reorg.md) — impact-alert full-screen / `GET /api/impact` /
  `applied_recommendations` / `impact_regressions` unbuilt → M19 (touches M16
  Impact pane).
- [C-1](../../../../reorg.md) — planner baseline/ratchet (M20) feeds the baseline
  FTR that `MeasureVerdicts` reads; until C-1 lands, `baseline_ftr` is project
  creation FTR.
- [M-7 / M-8 / M-9](../../../../reorg.md) (M8) — no registered metric reads
  impact/verdict data.

## Related

- [spec/screen/project-impact.md](../../../../spec/screen/) · [spec/pipeline/impact.md](../../../../spec/pipeline/) — legacy depth, folds here.
- [prd/02-objectives.md §05](../../../02-objectives.md) · [prd/03-personas.md §2,3,6](../../../03-personas.md).
- Crates: `crates/senseid/src/{verdicts.rs, model_insight.rs, tasks/handlers/verdicts.rs, api/handlers/project_detail.rs}`.