# Insights & Impact · M19

**Plane:** delivery · **Status:** [roadmap/03-delivery.md](../../../../roadmap/03-delivery.md) ·
**Serves:** team developer ([persona 2](../../../03-personas.md#2--team-developer--the-inheritor)),
maintainer ([persona 3](../../../03-personas.md#3--maintainer--the-curator)),
manager / delivery lead ([persona 6](../../../03-personas.md#6--manager--delivery-lead--the-roll-up))

## Purpose

Insights & Impact turns the signal stream into a **verdict on the assistant's
effectiveness** and a **narration the user can trust**. It measures whether sensei's
advice moved the workspace's FTR up or down, composes a honest headline/body/revision
for each case, and surfaces the evidence (the memory or recommendation that drove
it) so the user can verify. The job is **no fabricated consensus, no fabricated
zero** — if there's no signal, say nothing; if the delta is within ±0.05, say
neutral; if positive, **Reinforce** the source memory; if negative, **Retire** it (or
apply the concrete revision and re-measure). A verdict is not the end of the line: each
one leads to Reinforce or Retire, the two M9 verbs Impact drives (D65). The user feels
this as *I can see whether sensei helped, why, and what to do about it.*

> **Status honesty.** The verdict engine, the narration cache, and the model
> insight ranking are built, and so is the verdict's write-back to the source memory
> (positive → one `applied` outcome, negative → one `violated` outcome). **The
> `insights` table, `applied_recommendations`, `impact_regressions`, and
> `GET /api/impact` are unbuilt** — they appear in the legacy screen spec but have no
> DDL and no code. The per-workspace impact endpoint (`GET /api/workspaces/{id}/impact`)
> exists; the observatory-level `/api/insights` also exists. **The Reinforce / Retire
> next step on the Impact pane is not built:** the pane shows the verdict and a manual
> impact log (`Log impact`, then Success / Mixed / Failure), with no Reinforce or Retire
> control.

## In scope / out of scope

- In: `MeasureVerdicts` (recomputes FTR delta vs `baseline_ftr`, bands at ±0.05);
  the verdict variants `used` / `partial` / `ignored` → `positive` / `neutral` /
  `negative` with an idempotent outcome on the source memory; the next step each
  verdict offers (Reinforce / Retire, D65); the narration cache (key =
  sha256(kind + canonical json)); model insight ranking (winner / no winner logic);
  per-workspace impact endpoint; observatory tool-insights endpoint.
- Out: the `insights` table (does not exist in DDL); `applied_recommendations`
  table; `impact_regressions` table; `GET /api/impact` (unbuilt); the planner
  baseline/ratchet (C-1 → M20).

## Surfaces

- **API** — `GET /api/workspaces/{id}/impact` · `/impact-verdicts` (list/create) ·
  `/impact-verdicts/{verdict_id}` (decide) · `GET /api/insights` (observatory tool-insights).
- **Daemon** — `verdicts.rs` (`Verdict::from_ftr_delta`, `compose_headline`,
  `compose_body`, `suggested_revision`); `model_insight.rs`; `tasks/handlers/verdicts.rs`
  (`measure_verdicts` → `pg_store/metrics.rs::measure_pending_verdicts`, which calls
  `pg_store/memory.rs::{reinforce,challenge}_source_memory_for_rec`);
  `api/handlers/workspace_detail.rs` (impact endpoints).
- **App** — workspace Impact pane; observatory Insights lane.
- **Data** — `sensei.project_ftr_metrics` (view `project_metrics`), `sensei.memories` +
  `sensei.memory_outcomes` (the verdict's write-back), `activity.sessions`, `narration_cache` (key = sha256).

## Data invariants

- **Lead.** FTR delta bands at **±0.05** (5 percentage points). `MeasureVerdicts`
  reads `project_metrics.ftr_14d` and compares to `baseline_ftr`; > +0.05 →
  positive, < −0.05 → negative, else neutral.
- **Lead.** Every verdict leads to Reinforce or Retire (D65). The automatic half is
  built: a positive verdict records one `applied` outcome on the source memory
  (Reinforce: `strength += 0.5`, `reinforced_count += 1`, battle_tested at
  `strength >= 4.0` with no violations); a negative verdict records one `violated`
  outcome (`strength -= 0.7`; the memory goes to `challenged`, or to `archived` —
  Retire — once strength falls below 1.0) and carries a `suggested_revision`. Both are
  idempotent per recommendation (`rec:<id>` marker) and run through the
  `memory_outcome_apply` trigger; no new memory is created. Neutral records nothing. The
  user-facing half is **not built**: positive offers Reinforce, negative offers Retire
  (or apply the revision and re-measure), neutral offers Retire with keep as the default.
- **Lead.** No fabricated zero. When no models are captured, the models list is
  empty — never a fabricated vote or consensus panel.
- **Lead.** Narration cache key is `sha256(kind + canonical_json(inputs))`.
- **Lead.** Model insight ignores low-sample models; winner needs a clear lead and
  two qualifying models.
- **Lead.** Wire camelCase: `baselineFtr`, `currentFtr`, `deltaPp`, `verdict`,
  `headline`, `body`, `revision`, `sourceMemoryId`, `models[]`.

## Signals shown

| Element | Meaning | Worked example | Decision it drives / next step |
|---|---|---|---|
| Verdict badge | positive / neutral / negative | `positive · +7pp` | positive → Reinforce the source memory · negative → Retire it, or apply the revision and re-measure · neutral → Retire, or keep (default) |
| Headline | one-line lift or regression | `FTR rose to 72% (+7pp) — sensei__search` | none beyond the verdict badge's — candidate to cut as a separate signal |
| Body | the delta and the window | `FTR moved from 65% to 72% across the measurement window … Keep it.` | none beyond the verdict badge's — candidate to cut as a separate signal |
| Revision | concrete suggestion for negative | `Consider narrowing the scope of the accepted recommendation …` | apply the revision and re-measure; if it still regresses, Retire |
| Source memory | the memory the verdict reinforced or challenged | `m-12` | Reinforce or Retire it (the verdict's next step); once it helps, Share → (M9) |
| Models list | real models, or empty | `[{name: "sonnet-4", ftr: 0.78}, …]` or `[]` | switch to the model with the higher FTR |
| Trend markers | applied events on the FTR chart | markers at `applied_at` | open the applied event that moved the line |

## Done gate

- `MeasureVerdicts` computes the delta, bands correctly, and records the right outcome
  on the source memory once per recommendation.
- Every verdict the Impact pane shows offers its next step: Reinforce (positive),
  Retire (negative), Retire-or-keep (neutral) — not built.
- `compose_headline` / `compose_body` / `suggested_revision` produce stable,
  honest copy for all three verdicts.
- The narration cache hits on identical inputs; a changed input is a miss.
- Model insight ranks correctly and declines when the lead is too small or samples
  are too few.
- The workspace impact endpoint returns the verdict, the trend markers, and the
  source memory link.

```bash
curl -s http://localhost:7744/api/workspaces/$ID/impact | jq '.verdict, .headline, .models|length'
```

## Wrong gate

- **The verdict is `positive`/`negative` when `|delta| ≤ 0.05`** — band regression.
- **A positive verdict leaves the source memory untouched, or a negative verdict omits
  the revision** — the verdict's write-back regressed.
- **A verdict renders with no next step** — a positive verdict without Reinforce, or a
  negative one without Retire (D65).
- **Impact offers its own competing verbs** (promote · transfer) instead of Reinforce /
  Retire.
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
  FTR that `MeasureVerdicts` reads; until C-1 lands, `baseline_ftr` is workspace
  creation FTR.
- [M-7 / M-8 / M-9](../../../../reorg.md) (M8) — no registered metric reads
  impact/verdict data.

## Related

- [spec/screen/project-impact.md](../../../../spec/screen/) · [spec/pipeline/impact.md](../../../../spec/pipeline/) — legacy depth, folds here.
- [prd/02-objectives.md §05](../../../02-objectives.md) · [prd/03-personas.md §2,3,6](../../../03-personas.md).
- [M9 Memory & Knowledge](../../02-insight/09-memory-and-knowledge/README.md) — the
  Reinforce / Retire / Share → verbs (D65) and the promotion path (D56).
- Crates: `crates/senseid/src/{verdicts.rs, model_insight.rs, tasks/handlers/verdicts.rs, db/pg_store/metrics.rs, db/pg_store/memory.rs, api/handlers/workspace_detail.rs}`.