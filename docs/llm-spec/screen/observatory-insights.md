# 今 · Observatory · Insights (Learnings Triage)

**Segment:** 03 · Observatory — daily use
**Route:** `/insights`
**Source mockup:** [`lib/learnings-v2.jsx`](../../mockups/Sensei/lib/learnings-v2.jsx) → `LearningsTriage` (Option A — the wire target)
**App file:** `app/src/routes/(observatory)/insights/+page.svelte`

## Purpose

The user has real work to do today. They don't want to browse a
knowledge base — they want to know **what needs them, what's
working, and what's quiet, in that order**. The mockup answers with
three columns:

- **Now** (今) — violations, high-impact recommendations, top
  corrections. This is where the day's decisions live.
- **Soon** (近) — emerging patterns, medium-impact recs, challenged
  memories. Read once, revisit if the story clarifies.
- **Settled** (定) — battle-tested memories, sorted by strength.
  Browsable but low-noise. This is the "how we work here" shelf.

One project filter runs the whole page — the user picks a scope
and everything narrows. There are **no tabs** and **no sort
control** — the tri-column layout IS the sort. Every card has a
single-decision default (Apply / Review / Dismiss) with a highlighted
recommended action; the other verbs are one keystroke away. This is
the flagship instance of the **one decision, one default** theme.

Kanji is 今 — *now*.

## Data invariants

- `GET /api/insights` returns:
  ```json
  {
    "counts": { "now": N, "soon": N, "settled": N, "archived": N },
    "projects": [ { "id": "…", "name": "…", "kanji": "…" }, … ],
    "memories": [ Memory, … ],
    "recommendations": [ Recommendation, … ],
    "patterns": [ Pattern, … ],
    "corrections": [ Correction, … ]
  }
  ```
- Bucketing (server-side; the UI trusts the label):
  - **Now**: violations (`m.violated > 0 && m.state != "archived"`) +
    recs with `impact === "high"` + top 3 corrections
  - **Soon**: recs with `impact === "medium"` + emerging patterns
    (`p.kind === "emerging"`) + memories with `m.state === "challenged"`
  - **Settled**: memories with `m.state in ("battle-tested", "reinforced")`
    AND `m.violated === 0`, sorted by `strength` desc
- Every recommendation, memory, and pattern carries a stable `id`
  used for the accept/reject actions.
- Every card's user-facing text (title, body, impact sentence)
  reaches through [[pipeline/insight-copy]] — templated fallbacks
  are labelled as such.
- Actions map to daemon endpoints:
  - `POST /api/insights/recommendations/{id}/accept` → schedules
    a `MeasureVerdicts` follow-up
  - `POST /api/insights/recommendations/{id}/reject`
  - `POST /api/insights/memories/{id}/reinforce` / `challenge` /
    `archive`

## Signals shown

| Element | Value | Meaning |
|---|---|---|
| Project filter chip strip | project name + kanji + count | Narrows all three columns |
| Triage-stats row | `{now.count} · {soon.count} · {settled.count}` | Header micro-KPI |
| Now-column card: ViolationCard | violated memory + affected sessions | Highest signal — always focal in Now |
| Now-column card: RecCardSlim (impact=high) | rec title + impact sentence + Apply/Review/Dismiss | Universal Apply-verb; recommended action highlighted |
| Now-column card: CorrectionMini | one of the top-3 corrections | Signals cluster of same-shape mistakes |
| Soon-column card: RecCardSlim (impact=medium) | slower-decision rec | Deferred but not dismissed |
| Soon-column card: PatternMini | emerging pattern (>0 instances, unpromoted) | Not yet a rule; browsable |
| Soon-column card: ChallengedMini | memory contested by recent evidence | Needs a "keep / retire" decision |
| Settled-column list: memory row | scoped memory + strength bar | Sorted by strength desc |
| Empty-column copy | "nothing urgent." / "nothing brewing." / "nothing yet." | Honest quiet state, not "no data" |

## Done gate

- On Jerry's live data the three-column layout renders with real
  bucketing — every card ends up in the right column per the rules
  above.
- The project filter narrows all three columns in step; the
  `all` projection is the default; per-project scoping honors the
  `scope.project` field.
- Every recommendation card exposes the same three verbs (Apply /
  Review / Dismiss) with **one** verb highlighted as recommended
  (the mockup's one-decision-one-default theme).
- Accept on a rec triggers a `MeasureVerdicts` follow-up so the
  before/after FTR is measured and shown in `screen/observatory-impact`.
- Empty state text is the mockup's honest voice ("nothing urgent." /
  "nothing brewing." / "nothing yet."), not "no data" or "loading…".
- Every card's title + body comes through [[pipeline/insight-copy]]
  when the model is available; fallback templates otherwise.
- Dark mode: violation-red on paper-soft stays readable.

Optional check:
```
curl -s http://localhost:7744/api/insights | jq '{now: .counts.now, soon: .counts.soon, settled: .counts.settled}'
# expected: sum matches total memories + recs + corrections that pass their filters
```

## Wrong gate

- **A rec appears in both Now and Soon columns.** Server-side
  bucketing rules collided; the client should trust the server.
- **Apply on a rec silently succeeds but never triggers
  `MeasureVerdicts`.** The impact/before-after chain regressed
  (called out in [[pipeline/analyzer]]).
- **Column headers count 0-0-0 while cards render.** Count query
  and content query diverged.
- **Recommended verb varies from card to card without semantic
  reason.** The default-picker is inconsistent — should be a stable
  function of the card type.
- **A memory in Settled with `violated > 0`.** Bucketing rule
  broken — Settled requires zero live violations.
- **"Apply" changes the state but the Impact screen shows no
  measured verdict a day later.** MeasureVerdicts is not being
  scheduled OR the correlation join is broken.
- **The three verbs are separate buttons of equal weight (no
  recommended default).** Violates the one-decision-one-default
  theme.
- **All Now-column card copy reads the same** — insight-copy
  regression (see [[pipeline/insight-copy]] wrong-gate).

## Related

- [[pipeline/insights]] — the generator (bucketing rules live here)
- [[pipeline/memory]] — memories and their state machine
- [[pipeline/impact]] — before/after FTR for accepted recs
- [[pipeline/insight-copy]] — user-facing copy for every card
- [[screen/observatory-memories]] — the anatomy view for one memory
- [[screen/observatory-today]] — the "one thing" abstraction over this
