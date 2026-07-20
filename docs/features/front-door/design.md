---
name: front-door — design
updated: 2026-07-20
---

# front-door — design

> Behavior + structure. How the front door works: the intake flow, both
> surfaces, the axes→playbook rule matrix, the learning loop + auto-select, and
> the data contract. Self-contained — no need to open `operating-model.md` or
> `plan/*` to build or reason about this feature. Intent + axes are in
> [brief.md](brief.md); built-vs-partial-vs-gap is in
> [tests/acceptance.md](tests/acceptance.md).

## 1. Intake flow

Every work chunk enters through the same shape, on either surface:

```
freeform description → classify (axes) → recommend a playbook → recommend-and-confirm
```

- **Classify.** A local-first LLM classification via the gateway's `reasoning`
  chain reads the chunk description (plus project context — spine present or
  not, code-graph blast-radius) and assigns the three axes: `lifecycle`,
  `intent`, `risk`. A **heuristic fallback** classifies when the gateway is
  unavailable. Output is validated against the axis enums before use.
- **Recommend.** The classified axes are resolved against the rule matrix
  (§4) to a playbook + one-line rationale.
- **Recommend-and-confirm.** The recommendation is shown with its rationale;
  the human confirms or overrides. **High-risk chunks always require an
  explicit human confirm** — never auto-selected. **Low-risk** chunks may
  skip the confirm once the recommendation has earned trust (§5).

**The five UI states** (both surfaces implement these):

| State | Meaning |
|---|---|
| `describe` | Empty, ready — waiting for the freeform description. |
| `loading` | Classifying (gateway call / heuristic fallback in flight). |
| `recommended` | Recommendation card shown, awaiting confirm. |
| `recorded` | Confirmed and saved (or auto-selected and already recorded). |
| `error` | Classification or record failed — show the message, let the user retry. |

Sensei's read of the chunk (the three axes) is always shown back to the user
as a sanity check, not silently applied — this is what makes recommend-and-
confirm legible rather than a black box.

## 2. Two surfaces

Both surfaces are **Sensei** (per-user interaction layer) and render the same
underlying guidance content and recommendation; there is one intake, two
renderers.

### CLI / agent — `/sensei:intake`

The conversational surface, and the **primary in-session surface** while
actually coding. Orchestration: load the intake guide (frame + per-axis
prompts) → run the questionnaire → classify the chunk → recommend a playbook
→ recommend-and-confirm → persist the `playbook_run` → surface the chosen
playbook's `opening_tone` to prime the next stage. Session-start guidance
names `/sensei:intake` as the entry point for any new chunk; a light
non-blocking nudge hook exists to catch substantive work starting without an
intake, but hook **activation** is a separate, deferred, gated decision — it
ships off by default.

### App — the `/intake` Observatory screen

The desktop app's structured twin of the same conversation. Route
`app/src/routes/(observatory)/intake/+page.svelte`, reached from the
"Intake" rail anchor (first item — the front door leads the Observatory,
the per-user home).

Flow: freeform textarea → `POST /api/playbook/recommend` with `preview: true`
(classify + recommend, nothing persisted) → recommendation card (title,
rationale, opening tone, the three inferred axes as chips, a trust badge when
`auto_select` is true) → **confirm-persist**: a Confirm button re-sends the
already-classified axes with `confirm: true`, persisting exactly one
`playbook_run` row. When `auto_select` is true the app fires the confirm
automatically and announces the trust badge instead of waiting for a click;
high-risk chunks never carry `auto_select: true`, so they always show the
explicit Confirm button.

**Session-less by design.** The app path passes the app's active session id
if one exists, else `null`. A session-less run is still recorded (visible in
a future decision log) but is **not FTR-attributed** — the learning loop
(§5) only learns from live coding sessions, which today means the CLI/agent
path. The app's recommendation is advisory: it tells the user what sensei
would do, without requiring a coding session to back it.

## 3. Playbook catalog

Six playbooks today, DB-backed and runtime-extensible (`sensei.playbooks`,
text primary key `name` — adding a method later is an INSERT, not a release).
Values below are the seed data (`database/import/staging/playbooks.jsonl`):

| name | title | when it's recommended | opening tone |
|---|---|---|---|
| `vibe` | Vibe / spike | Greenfield, objective fuzzy — explore then extract learnings (discardable). | "Explore fast and loose; capture what you learn, keep nothing you cannot justify." |
| `mockup_first` | Mockup-first | Greenfield, UX-heavy — design the surface before the spec. | "Start from the mockup; let the UI shape the spec." |
| `spec_driven` | Spec-driven | Clear objective + high blast-radius — force a deep design first. | "Slow down: write the design, enumerate edge cases, before any code." |
| `gsd` | Get stuff done | Known feature, low risk — lean plan then build. | "Lean plan, then build; keep it tight." |
| `change_flow` | Change-flow | Stable product enhancement — impact analysis then targeted design. | "Map impact first; design the smallest change that lands the value." |
| `debug_flow` | Debug-flow | Stable product bug — reproduce, fix, add a regression test. | "Reproduce first; fix; lock it with a regression test." |

`title`, `when_to_use` and `opening_tone` are exactly what both surfaces
render — the catalog table above and the recommendation card read from the
same rows.

## 4. Axes → playbook rule matrix

The three axes (`lifecycle`: `greenfield`/`stable`; `intent`: `explore`/
`ux`/`feature`/`enhancement`/`bug`; `risk`: `low`/`high`) are resolved
against an ordered rule set (`sensei.playbook_rules`, seeded from
`database/import/staging/playbook_rules.jsonl`). A rule matches when every
non-wildcard axis on the rule equals the corresponding classified axis value;
the **highest-priority matching rule wins**. `*` below = wildcard (empty
string in the seed data = matches any value of that axis).

| # | lifecycle | intent | risk | → playbook | priority |
|---|---|---|---|---|---|
| 1 | * | * | high | `spec_driven` | 100 |
| 2 | greenfield | explore | * | `vibe` | 60 |
| 3 | greenfield | ux | * | `mockup_first` | 60 |
| 4 | stable | bug | * | `debug_flow` | 60 |
| 5 | stable | enhancement | * | `change_flow` | 50 |
| 6 | * | feature | low | `gsd` | 40 |
| — | *(no match)* | | | **default → `gsd`** | — |

Highest-priority matching rule wins (rule 1's high-risk wildcard outranks
every intent-based rule, so a high-blast-radius chunk always forces
`spec_driven` regardless of intent or lifecycle). Cells not covered by rules
1–6 fall through to the default (`gsd`) with a "no rule matched" rationale —
for example `stable+ux+low`, `greenfield+feature+low`, and
`greenfield+enhancement+*` have no explicit rule today. The full gap
analysis (which combos are uncovered, and why) is recorded in
[tests/acceptance.md](tests/acceptance.md) — a later task, not re-derived
here.

## 5. Learning loop + auto-select

**Attribution.** Each confirmed `playbook_run` carries a nullable `outcome` /
`outcome_ftr`. The analyzer's `LearnPlaybooks` global pass (a stage in
`analyze_project`, run hourly alongside the other global passes) attributes
outcomes after session enrichment:

```
recommend playbook → confirm/override → chunk runs → session gets FTR-scored
→ attribute FTR back onto the playbook_run → re-weight the rule set
```

FTR (first-turn resolution — did the session land with zero corrections) is
the signal, joined from the session the run's `session_id` points at. Only
**confirmed** runs with a live, FTR-scored session are attributed —
session-less app runs and unconfirmed recommendations are excluded.

**Re-weighting.** For each rule, the attributed runs matching its axes are
aggregated into an observed FTR rate; the rule's `priority` is recomputed as
`base_priority` plus a bounded adjustment toward a fixed target FTR (a good
FTR nudges priority up, a bad one nudges it down, clamped so no rule can run
away) — deterministic and idempotent, never accumulated across passes.

**Proposing new rules.** Where a different playbook clearly out-performs the
one currently recommended for an axes-combo, the pass proposes a new
`source='learned'` rule (disabled by default, invisible to the resolver
until a human accepts it) rather than silently rewriting behavior.

**Auto-select-on-trust.** At recommend time, for **low-risk** chunks only,
sensei checks the chosen playbook's live FTR track record for the exact axes
combo: `n` (sample count) and `ftr` (FTR rate). When `risk == low` **and**
`n >= 10` **and** `ftr >= 0.8`, the recommendation auto-confirms instead of
waiting for the human: the run is recorded and sensei announces what it did
("auto-selected `<playbook>` — reliable for this kind of chunk, FTR `<ftr>`
over `<n>` runs. Say 'change' to pick another."). This is reversible — the
human can still override. **High-risk chunks never auto-select**; the trust
check isn't even run for them, so they always land in the `recommended` state
awaiting an explicit confirm.

## 6. Data contract

Two daemon endpoints drive both surfaces; field names below are what UI copy
can rely on.

**`GET /api/playbook/guide`**
```json
{
  "frame": "…the intro line grounding the intake…",
  "axes": [ { "axis": "lifecycle|intent|risk", "prompt": "…", "help": "…" } ],
  "playbooks": [ { "name": "…", "title": "…", "when_to_use": "…", "opening_tone": "…" } ]
}
```

**`POST /api/playbook/recommend`**
```json
{
  "playbook": "debug_flow",
  "rationale": "A bug in a stable product — reproduce, fix, regress.",
  "lifecycle": "stable",
  "intent": "bug",
  "risk": "low",
  "opening_tone": "Reproduce first; fix; lock it with a regression test.",
  "when_to_use": "Stable product, bug — reproduce, fix, add a regression test.",
  "auto_select": false,
  "trust": { "n": 0, "ftr": 0.0 }
}
```

**Flags:**
- **`preview: true`** — classify + recommend only, skip persistence entirely.
  This is the leg the app's recommend step uses, so the recommendation card
  can be shown before anything is written.
- **`confirm: true`** — records exactly one `playbook_run` row, reusing the
  already-classified axes (no re-classification, no second gateway call).
  The CLI and the app each call this exactly once per chunk, avoiding a
  double-insert.

The app path is **session-less by design** (§2): it passes the app's session
id if one exists, else `null`, so app-confirmed runs are recorded but not
FTR-attributed.

## 7. Depth note

This feature is shipped-but-partial: the intake flow, both surfaces, the
rule matrix, the learning loop, and auto-select described above are built to
varying degrees of completeness (some shipped-and-released, some
shipped-not-yet-released, some still gapped). The built vs. partial vs.
known-gap breakdown lives in [tests/acceptance.md](tests/acceptance.md).
