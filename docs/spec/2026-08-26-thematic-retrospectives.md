# Thematic retrospectives — repo-level and cross-repo

Status: draft
Extends: `2026-08-20-transcript-process-quality-analyzer.md`,
`2026-08-20-insight-acceptance-materialization.md`,
`2026-08-26-session-facets-and-retrospective-report.md`

## 1. Problem

sensei measures a great deal and explains almost none of it thematically. On the
live daemon DB today:

| | rows |
|---|---:|
| `sensei.narration_cache` | 16,753 |
| — of which `metric_day_explainer` | 11,985 |
| — `community_description` | 2,423 |
| — `tool_workhorse` / `tool_dormant` | 1,548 |
| — `session_retrospective` | 505 |
| — `insight_recurring_pattern` | 56 |

Every one of those is **single-subject**: one metric on one day, one code
cluster, one tool, one session. Nothing answers a question that spans them.

The questions people actually ask in a retrospective are cross-cutting:

- *At what stage is the rework happening?*
- *Where is the bottleneck in getting from intent to shipped?*
- *What do we keep struggling with?*
- *What should we change — and can it become a rule, a skill or an agent?*

None is answerable from an explainer about `rework_ratio` on the 14th.

## 2. What already exists (verified against the live DB)

This design adds a layer; it does not add a stack. Confirmed present and
populated:

- **151 sessions**, 16 repos, 4 ACPs, 310,751 assistant events.
- **`sensei.metrics`** — a 29-metric catalogue that already includes
  `rework_ratio`, `rework_density`, `ftr`, `throughput`,
  `time_to_useful_result`, `session_duration`, `spec_depth`,
  `spec_deviation_rate`, `incomplete_analysis_llm_rate`,
  `refuted_finding_rate`, `context_pressure_rate`, `interruption_rate`,
  `churn_rate`, `cost_per_result`.
- **`sensei.metric_facts`** (15,636) and `repository_metrics` (15,636),
  `project_metric_daily/weekly/monthly/quarterly`, `project_metric_trend`,
  `metric_ratings` (446).
- **`props.process`** on 128 of 151 sessions — the four LLM process judgments —
  with **535 grounding quotes** in `activity.session_process_evidence`.
- **`sensei.narration_cache`** — already LLM-authored and cached by `facts_hash`,
  carrying `model_provider`/`model_id`. The generation-and-cache mechanism we
  need is already here.
- **Insight acceptance → materialization**: P-A (governance rule) and P-B
  (project skill/agent file, consent-gated) have landed.

**So the thematic layer is new copy over existing facts, feeding an existing
acceptance path.** Nothing below proposes a second metric set, a second cache,
or a second way to turn an insight into an artifact.

## 3. The one real data gap: stage

"At what stage is there more rework" cannot be answered today. `activity.runs`
carries `current_phase`, but there are **10 runs against 151 sessions** — the
other 141 have no stage at all. `sessions.props.workflow_phase` is documented
and never written.

### D1 — Stage, recorded when known, inferred otherwise, never conflated

```
sensei.work_stage  enum
  ('explore', 'analyze', 'plan', 'build', 'verify', 'fix', 'operate')
```

Added to the facet record, because for most sessions it IS a judgment:

```
activity.session_facets
  + stage         work_stage
  + stage_source  stage_source   -- 'recorded' | 'inferred'
```

**Correction, found while building this.** The design above assumed
`activity.runs.current_phase` was a recorded workflow phase and could be
resolved against the inference. It is not: the column is free text and on the
live daemon it holds feature descriptions — `"P4 · stall signal — revive +
no-re-stall proof"` — not a stage. `activity.runs` also has no link to
`activity.sessions` (it carries `dojo_session_id`, a different grain).

So today **every stage is inferred**, and the resolving view has one source,
which is ceremony rather than design. `stage_source` still exists and still
matters: it is what lets a recorded source be added later without making the
existing rows ambiguous, and it keeps every rollup able to say the stages were
read rather than declared. The view lands when there is a second source to
resolve — see open question 3.

`stage_source` is not decoration. A rollup that says "38% of rework happens in
`build`" means something different if the stage was inferred by a model than if
the developer declared it, and every surface that shows a stage rollup must be
able to say which. Fail-closed: a session whose stage the analyzer will not
commit to is `NULL` and is excluded from stage rollups rather than bucketed into
a default.

## 4. The four themes, and how each is computed

Each theme is a **query over existing facts**, then one LLM call to narrate it.
The narration is cached in `narration_cache` under a new `kind`, keyed on the
`facts_hash` of the query result — so identical facts never pay twice, and a
changed fact regenerates.

### T1 — Where rework concentrates

`rework_ratio` / `rework_density` / `corrections`, grouped by
`activity.session_stage`, then by repo. The finding is the *shape*: rework
piling up in `build` says the plan was thin; rework in `verify` says the
acceptance criteria were discovered late.

### T2 — Where the bottleneck is

`time_to_useful_result`, `session_duration`, `throughput` and
`context_pressure_rate` across stages. A bottleneck is a stage whose elapsed
share is disproportionate to the work that leaves it — measured, not asserted.

### T3 — What we keep struggling with

`session_facet_tags where kind='friction'` plus the four `props.process`
judgments, weighted by recurrence *across sessions and across repos*. A struggle
seen in one repo is a repo finding; the same struggle in three is a practice
finding.

### T4 — What to change

The synthesis, and the only theme that produces a proposal rather than a
description. Every proposal carries an `action_type` the existing
materialization map already understands, so accepting one writes a rule, a skill
or an agent by the path built in P-A/P-B.

## 5. Two scopes, deliberately different

| | repo retrospective | cross-repo retrospective |
|---|---|---|
| grain | one `repo_key` | all repos for the persona |
| answers | what is happening in THIS codebase | what is true of how we work |
| threshold | a pattern in this repo's sessions | recurrence in **≥ 3 repos** |
| materializes to | project rule / project skill / project agent | governance rule / shared pack rule |
| cadence | on demand + weekly | monthly |

The threshold is the point. A cross-repo recommendation that fires off one
repo's habit is how a local preference becomes everyone's mandatory rule. P-C of
the acceptance spec already defines recurrence detection for exactly this; this
design is its input.

## 6. Generation discipline

Carried over verbatim from the two analyzers that already work this way, and
from what the offline report tool learned the hard way:

- **Grounded or dropped.** A theme narration cites the sessions it rests on; one
  that cites a session outside its own fact set is discarded, not printed.
- **No lookup tables.** The offline retrospective began as advice keyed on a
  friction name and produced four identical paragraphs across five people.
  Gating it on a threshold made the numbers vary and left the sentences fixed.
  Theme copy is synthesised per fact set or it is not written.
- **Say what was excluded.** A theme computed over 38 of 151 sessions says so.
  A rollup that silently drops the stage-less majority reads as a finding about
  everyone.
- **Honest-empty.** Fewer than N sessions in a stage produces no claim about
  that stage, not a claim with a wide error bar.

## 6a. Grain discipline — measured, not assumed

Found while checking whether the themes hold up on real data, and load-bearing
enough to be a rule rather than a note.

The per-repo process rollup tells a clean story:

| repo | sessions | avg spec depth | deviated | refuted | shallow |
|---|---:|---:|---:|---:|---:|
| torii | 16 | 2.9 | 56% | 44% | 50% |
| sensei | 37 | 3.2 | 41% | 19% | 19% |
| rokkit | 17 | 4.3 | 18% | 12% | 24% |

Thin plans, more drift. It is the obvious narration and a synthesiser would
write it. But at SESSION grain the same relationship is weak — Pearson −0.23
over 97 sessions, and depth 3 deviates *more* (70%) than depth 2 (43%). The
repo-level pattern is real; the session-level mechanism it implies is not
established.

sensei's own correlation engine reports the same pair at **−0.54 Spearman** —
but on DAY-grain rates, which is a third measurement again. All three numbers
are correct and they are claims about different things.

**Rule: a theme narrates at the grain it was measured at, and says which.**
"Repos with deeper plans drift less" is supportable. "Writing a deeper plan
makes *this session* drift less" is not, from the same number. A retrospective
that blurs them tells someone to change a behaviour on evidence that never
measured that behaviour.

## 6b. Reuse the correlation engine; do not add a second one

`crate::correlate` already exists, is wired through `pg_store::metrics`, and is
better than anything this layer would write in passing:

- **Spearman**, because a token count spans millions and a ratio sits in [0,1],
  so one outlier day would dominate Pearson.
- **`MIN_RHO` 0.40, `MIN_PAIRS` 20** — under those it stays quiet.
- **Definitional suppression** via each metric's `derives_from`, because
  `tokens_in_per_day / tokens_per_day` at ρ=1.00 is arithmetic, not an insight.

T1 and T2 consume its output. They do not compute their own coefficients.

Likewise `insight_recurring_pattern` (56 rows) already narrates recurring
churn. T3 extends that vocabulary rather than replacing it.

## 6c. Measured on real data (86 sessions, release daemon)

P1 deployed and drained through the on-demand `process/analyze` endpoint. Every
stage below was inferred from a transcript by the shipped binary — this is what
the vocabulary actually produces, not what it was hoped to produce.

| stage | n | deviated | shallow | corrections | avg depth |
|---|---:|---:|---:|---:|---:|
| build | 26 | 38% | 31% | 5 | 3.2 |
| analyze | 26 | 38% | 23% | 8 | 3.3 |
| plan | 19 | 26% | 21% | 3 | 3.7 |
| verify | 7 | 29% | 29% | 3 | 3.7 |
| fix | 6 | 33% | 17% | 0 | 3.0 |
| operate | 2 | 0% | 0% | 0 | 3.5 |
| explore | 1 | 0% | 100% | 0 | 2.0 |

Three findings that change the design:

**Open question 1 is answered: collapse `explore`.** One session in 86. The
analyzer cannot separate orienting-in-unfamiliar-code from diagnosing, and asked
to choose it picks `analyze` — which is the right call under "do not guess", but
it means the value earns nothing. `operate` at 2 is on the same edge. Fold
`explore` into `analyze` and keep `operate` only while deployment work is rare
enough to be worth naming.

**Rework does concentrate, mildly.** Corrections sit in `analyze` (8) and
`build` (5); deviation is highest in those same two at 38% against `plan`'s 26%.
`plan` also carries the deepest specs (3.7). The shape is consistent with the
day-grain correlation the engine already reports — and per §6a it is stated at
the grain measured: *sessions doing planning deviate less*, NOT *planning more
would make this session deviate less*.

**Repo × stage is still too sparse.** 7 of 31 cells reach n≥5. A repo
retrospective can say what stages a repo's sessions were, but cannot yet compare
rework ACROSS stages within one repo without falling below the threshold §6
already sets. T1 lands cross-repo first; the per-repo cut waits for volume.

## 7. Phasing

- **P1 — stage attribution.** ✅ DONE — deployed, drained, measured (§6c).
  Original note: D1 plus inference in the existing facet pass. No
  new LLM call: the analyzer already reads the transcript. Unblocks T1 and T2.
- **P2 — repo retrospective.** T1–T3 at repo grain, one new `narration_cache` kind,
  surfaced where project insights already are.
- **P3 — T4 proposals** wired to the existing accept → materialize path.
- **P4 — cross-repo**, with the ≥3-repo threshold, feeding P-C.
- **P5 — cadence**: weekly per repo, monthly cross-repo, on the existing task
  worker rather than a new scheduler.

## 8. Open questions

1. **Stage granularity.** Seven stages may be more than the transcripts can
   distinguish. If the analyzer cannot separate `explore` from `analyze`
   reliably, collapse them rather than record a coin flip.
2. **Whose retrospective is the cross-repo one?** Per persona today. A dōjō with
   several members will want a team grain, which is the same query with a
   different `WHERE` — but the privacy question is not the same one.
3. **Do sessions need an explicit stage prompt?** Inference covers history;
   asking once per session would make it fact. That is a UX cost against a data
   quality gain, and it is a product decision rather than a technical one.
