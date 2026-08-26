# Checkpoint

**Slice:** thematic retrospectives — repo and cross-repo (spec `docs/spec/2026-08-26-thematic-retrospectives.md`)

## Why

sensei measures a lot and explains none of it thematically. Of 16,753 cached insights, 11,985 are one metric on one day. Nothing answers *where is the rework*, *where is the bottleneck*, *what do we keep struggling with*.

## Done

- **Spec written**, grounded in the live DB. Key finding: this is new *copy over existing facts* feeding an *existing* acceptance path — the 29-metric catalogue, `metric_facts` (15,636), LLM-authored `insight_copy` cached by `facts_hash`, and insight→rule (P-A) / →skill/agent (P-B) all already exist.
- **P1a — stage schema.** `sensei.work_stage`, `sensei.stage_source`, both columns on `activity.session_facets` under a check that they are set together. Applied; `dbd diff --exit-code` clean.
- **P1b — stage inference.** A fifth key in the call the process analyzer already makes. Values outside the enum are dropped, not coerced; the prompt tells the model to answer null rather than guess. 2412 daemon tests, clippy 0.
- **Upsert semantics verified against the live DB**: re-inference updates itself, a `recorded` stage is never overwritten by re-analysis, and a stage with no source is rejected by the constraint.

## Next

1. **Deploy to see real stages.** The running daemon is the pre-change binary, so `activity.session_facets` is still empty. `make install-debug` + restart, clear `process_analyzed_at` on a batch, confirm stages land and check the distribution is plausible before trusting any rollup.
2. **P2 — repo retrospective**: T1 (rework by stage) and T2 (bottleneck) as queries over `metric_facts` + stage, narrated into a new `insight_copy` kind.
3. **P3** — T4 proposals wired to the existing accept → materialize path.
4. **P4** — cross-repo at the ≥3-repo recurrence threshold, feeding P-C.

## Corrections made to the spec while building

`activity.runs.current_phase` is **not** a workflow phase — it is free text holding feature descriptions ("P4 · stall signal — revive…"), and `activity.runs` has no link to `activity.sessions`. The design assumed it could be resolved against the inference; it cannot, so every stage is inferred today and the resolving view was dropped rather than shipped with one source.

## Known-broken / caveats

- No stage data exists yet — inference needs the daemon redeployed (item 1).
- 7 stages may be more than a transcript can distinguish; collapse rather than record a coin flip if `explore`/`analyze` prove inseparable.
- Cross-repo grain is per-persona. A dōjō team grain is the same query with a different `WHERE`, but not the same privacy question.
