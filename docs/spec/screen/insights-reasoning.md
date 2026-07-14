# 論 · Observatory · Insights · Reasoning panel

**Segment:** 03 · Observatory — daily use
**Route:** `/insights/{id}/reasoning` (drawer / secondary route on the Insights screen)
**Source mockup:** derived from [`lib/observatory/mcp-replay-insights.jsx`](../../mockups/Sensei/lib/observatory/mcp-replay-insights.jsx) MOE section
**App file:** `app/src/routes/(observatory)/insights/[id]/reasoning/+page.svelte`

## Purpose

For the highest-stakes recommendations (a memory that might
contradict an existing one, a regression that needs analysis, a
pattern promotion), the MOE panel in
[[pipeline/inferencing]] runs a propose → challenge →
synthesize debate. This screen makes that debate **visible** —
the user can see why the pipeline arrived at its answer, whether
the models agreed, and what the disagreements were.

Not a default view — a drawer / secondary route the user opens
when they want to inspect the reasoning.

Kanji is 論 — *debate / discourse*.

## Data invariants

- Each MOE consensus write persists `sensei.reasoning_traces`:
  - `id`, `insight_id` (fk into the underlying rec / memory /
    verdict), `chain_name`, `models_used` text[],
    `stage`, `raw_trace` jsonb (per-model output),
    `synthesized_answer` text, `confidence` enum,
    `disagreements` text[], `run_at`, `duration_ms`.
- `GET /api/insights/{id}/reasoning` returns the trace.

## Signals shown

| Element | Value |
|---|---|
| Header | insight title + confidence chip |
| Models used | pill list of models involved |
| Propose row | per model — the proposed answer |
| Challenge row | per model — what it noticed in the others |
| Synthesize row | final synthesized answer |
| Disagreements section | bullet list of what the models didn't agree on |
| Raw trace toggle | reveals the full jsonb per stage per model |
| Confidence indicator | high / medium / low + the rule that produced it |

## Done gate

- Every MOE-produced insight (memory consolidation, negative-
  verdict analysis, pattern promotion) writes a
  `reasoning_traces` row. `sensei.reasoning_traces` is a new
  table required by this screen — see [[pipeline/inferencing]]
  MOE section.
- The reasoning drawer opens without a full navigation from the
  Insights screen; deep-link at
  `/insights/{id}/reasoning` opens direct.
- Opening the drawer for a MOE-produced insight shows ≥ 2 rows
  under `models_used`.
- Disagreements are highlighted when present (non-empty
  `disagreements` array on the trace).
- Raw trace is available for the technically-inclined user
  (toggle in the drawer).

Optional check:
```
curl -s http://localhost:7744/api/insights/{id}/reasoning \
  | jq '{stage: .stage, models: (.models_used | length),
         disagreements: (.disagreements | length),
         confidence: .confidence}'
```

## Wrong gate

- **Reasoning trace missing for a high-confidence insight.**
  Persistence path skipped.
- **Confidence = high when models actually disagreed.** Rule
  wrong.
- **Same model listed twice in `models_used`.** Panel config
  duplication.
- **Raw trace shows sanitised text.** Sensitive redaction was
  too aggressive; the technical user needs the full evidence.

## Related

- [[pipeline/inferencing]] — MOE consensus source
- [[pipeline/insights]] — insight that owns the trace
- [[pipeline/memory]] — consolidation traces
- [[pipeline/impact]] — negative-verdict analysis traces
- [[screen/observatory-insights]] — parent screen
