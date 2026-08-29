# 今 · Observatory · Insights (Learnings Triage)

**Segment:** 03 · Observatory — daily use
**Route:** `/insights`
**Source mockup:** [`lib/observatory/learnings-v2.jsx`](../../mockups/Sensei/lib/observatory/learnings-v2.jsx) → `LearningsTriage` (Option A — the wire target)
**App file:** `app/src/routes/(observatory)/insights/+page.svelte`

## Purpose

The user has real work to do today. They don't want to browse a
knowledge base — they want to know **what needs them, what's
working, and what's quiet, in that order**. The mockup answers with
three columns:

- **Now** (今) — violations, high-urgency recommendations, top
  corrections. This is where the day's decisions live.
- **Soon** (近) — emerging patterns, medium-urgency recs, challenged
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

- `GET /api/insights` is a **new endpoint** (not yet in routes.rs; must be built).
  It is a server-side aggregator that bundles three sources into Now / Soon / Settled
  columns, cross-project by default. Pass `?project=<name-or-uuid>` to scope all
  three columns to a single project. It returns:
  ```json
  {
    "counts": { "now": N, "soon": N, "settled": N },
    "projects": [ { "id": "…", "name": "…", "kanji": "…" }, … ],
    "memories": [ Memory, … ],
    "recommendations": [ Recommendation, … ],
    "patterns": [ Pattern, … ],
    "corrections": [ Correction, … ]
  }
  ```
- Sources and bucketing (server-side; the UI trusts the label):
  - **recommendations** — `inference.recommendations` WHERE `status = 'pending'`,
    bucketed by `urgency`: `'high'` → Now, `'medium'` → Soon, `'low'` → Settled.
    Each card carries `project_id` + project name (for the action URL and scope chip).
  - **memories** — `sensei.memories`, bucketed by `status` (`memory_status` enum):
    - violations (`m.violated_count > 0 && m.status != 'archived'`) → Now
    - `'proposed'` → Soon
    - in-force (`'active'` / `'reinforced'` / `'battle_tested'`) → Settled, sorted by `strength` desc
  - **patterns** — `inference.detected_patterns`, bucketed by `lifecycle`:
    `'suggested'` → Soon, `'rule'` → Settled.
  - **corrections** — top 3 by count from `inference.corrections` → Now.
- Every recommendation, memory, and pattern carries a stable `id`
  used for the accept/reject actions.
- Every card's user-facing text (title, body, impact sentence)
  reaches through [[pipeline/narration-cache]] — templated fallbacks
  are labelled as such.
- Actions map to daemon endpoints (each card carries its own `project_id`):
  - Apply → `POST /api/projects/{project_id}/recommendations/{rec_id}/accept`
    (already exists in `project_detail.rs`; schedules a `MeasureVerdicts` follow-up
    so before/after FTR is captured automatically)
  - Dismiss → `POST /api/projects/{project_id}/recommendations/{rec_id}/reject`
  - Review → navigate to the recommendation/impact detail — no write call
  - Memory write-actions (reinforce / challenge / archive) are **display-only /
    deferred** for this screen. Only `/api/knowledge/memories/{id}/promote` exists
    today; the other endpoints are not yet built. Memories in Settled are shown as
    adopted learnings, not triaged here.

## Signals shown

| Element | Value | Meaning |
|---|---|---|
| Project filter chip strip | project name + kanji + count | Narrows all three columns |
| Triage-stats row | `{now.count} · {soon.count} · {settled.count}` | Header micro-KPI |
| Now-column card: ViolationCard | violated memory + affected sessions | Highest signal — always focal in Now |
| Now-column card: RecCardSlim (urgency='high') | rec title + impact sentence + Apply/Review/Dismiss | Universal Apply-verb; recommended action highlighted |
| Now-column card: CorrectionMini | one of the top-3 corrections | Signals cluster of same-shape mistakes |
| Soon-column card: RecCardSlim (urgency='medium') | slower-decision rec | Deferred but not dismissed |
| Soon-column card: PatternMini | `lifecycle = 'suggested'` pattern from `inference.detected_patterns` (>0 instances, unpromoted) | Not yet a rule; browsable |
| Soon-column card: ChallengedMini | memory contested by recent evidence | Needs a "keep / retire" decision |
| Settled-column list: memory row | scoped memory + strength bar | Sorted by strength desc |
| Empty-column copy | "nothing urgent." / "nothing brewing." / "nothing yet." | Quiet-state copy from the mockup — not "no data" or "loading" |

## Done gate

- On Jerry's live data the three-column layout renders with real
  bucketing — every card ends up in the right column per the rules
  above.
- The project filter narrows all three columns in step; the
  cross-project view is the default; passing `?project=<name-or-uuid>`
  scopes all three columns to a single project.
- Every recommendation card exposes the same three verbs (Apply /
  Review / Dismiss) with **one** verb highlighted as recommended
  (the mockup's one-decision-one-default theme).
- Accept on a rec triggers a `MeasureVerdicts` follow-up so the
  before/after FTR is measured and shown in `screen/observatory-impact`.
- Empty state text is the mockup's voice ("nothing urgent." /
  "nothing brewing." / "nothing yet."), not "no data" or "loading…".
- Every card's title + body comes through [[pipeline/narration-cache]]
  when the model is available; fallback templates otherwise.
- Dark mode: violation-red on paper-soft stays readable.

Optional check:
```
curl -s http://localhost:7744/api/insights | jq '{now: .counts.now, soon: .counts.soon, settled: .counts.settled}'
# expected: .now >= 1  (337 pending recs exist, so Now must be non-empty)
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
- **A memory in Settled with `violated_count > 0`.** Bucketing rule
  broken — Settled requires zero live violations.
- **"Apply" changes the state but the Impact screen shows no
  measured verdict a day later.** MeasureVerdicts is not being
  scheduled OR the correlation join is broken.
- **The three verbs are separate buttons of equal weight (no
  recommended default).** Violates the one-decision-one-default
  theme.
- **All Now-column card copy reads the same** — narration-cache
  regression (see [[pipeline/narration-cache]] wrong-gate).

## Related

- [[pipeline/insights]] — the generator (bucketing rules live here)
- [[pipeline/memory]] — memories and their state machine
- [[pipeline/impact]] — before/after FTR for accepted recs
- [[pipeline/narration-cache]] — user-facing copy for every card
- [[screen/observatory-memories]] — the anatomy view for one memory
- [[screen/observatory-today]] — the "one thing" abstraction over this
