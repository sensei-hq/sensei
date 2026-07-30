# Dōjō inbox — mockup-fidelity spec (existing flow, no data-model changes)

**Mockup (source of truth):** `docs/mockups/Sensei/lib/dojo2/dojo2-app.jsx` → `DojoApp2 start="you"`
(`ScrInbox` + `RunDetail`). Render it with a harness (`mount(DojoApp2,{start:'you'})`) and
diff element-by-element. Reference screenshots: `~/Desktop/mockup.png`.

**Rule for this doc:** every field maps to data the EXISTING flow already delivers. No new
columns, no new federation, no zero-knowledge changes. Where the flow does not deliver a
field, it is called out explicitly (§4) — not faked, not "honest-empty"-skipped.

---

## 1. The flow (verified)

```
sensei:plan  →  register_plan (MCP)  →  daemon stores activity.runs.plan_graph
   PlanGraph { goal, phases[ { title, tasks[ { id,title,agent,model,spec_ref,summary,state,deps } ] } ] }

daemon publish_run  →  dojo.relay_sessions   (run-level: title, goal, status, progress_done/total,
                                              current_phase, current_feature, last_event_at, started_at, heartbeat_at)
                    →  dojo.relay_segments   (graph projected: per node title, agent, model, spec_ref,
                                              state, is_gate, gate_severity, parent_id, seq)
                    →  dojo.relay_inbox       (asks: kind, payload{prompt, options})

dojo inbox      (inbox)/+layout.ts : listRuns(relay_sessions) + listGates(relay_inbox)   ← NO segments
dojo run-detail runs/[id]/+page.ts : getSegments(relay_segments) + listRuns + listGates  ← HAS the graph
```

**Key fact:** the graph's rich fields (model, agent, spec, per-phase states) reach the dōjō
as `relay_segments`. The run-detail already fetches them. The inbox does NOT fetch segments,
which is why its rows have no pips — the data exists in `dojo.relay_segments`, the inbox load
just needs to read it (per §3.A). This is reading existing data, not a model change.

---

## 2. Root cause of the rework cycle (so it stops)

1. Built the panels against `relay_sessions` (thin status feed) and ignored the registered
   graph (`relay_segments`) — the mockup's shape IS the graph shape.
2. Tested against leftover demo rows / bare `start_run`s with no graph → nothing to render →
   mis-read the empty result as a layout problem. (Demo rows now deleted.)
3. Proposed data-model changes (federate model/edits, extend PlanGraph) instead of using the
   flow. No model change is needed — everything below uses existing tables.
4. Verified by gestalt, not field-by-field. See [[feedback_mockup_faithful_replication]].

**To verify this spec:** a run with a REAL registered graph must exist — run `sensei:plan` +
`register_plan` for the sensei project, let it federate, then diff against the mockup.

---

## 3. Element → data mapping (build to this exactly)

### A. Left rail — `ScrInbox` + `InboxRow` (mockup K2InboxRow)

| # | Element | Mockup | Existing source | Build action |
|---|---|---|---|---|
| A1 | status dot | tone by state | `relay_sessions.status` | ✓ already |
| A2 | top-left = **repo** | `lumen-auth` | run's project (see §4 — NOT in dojo today) | §4 decision |
| A3 | top-right = age | `2m` | `last_event_at` | ✓ already |
| A4 | title (2-line) | task title | `goal ?? title` | fix `toKitRun`: **stop the `project: current_feature ?? title` fallback** so the title never prints twice |
| A5 | why-line | `3 need you` / `no heartbeat` | gates count / status | ✓ already |
| A6 | **plan-pips** | per-phase colored strip | `relay_segments` phase states | inbox load must fetch segments per run (§3.A.i) → build `KitPlan` → `PlanPips` renders |
| A7 | done/total | `5/12` | `progress_done/total` | ✓ already |
| A8 | rail container | pad 32/24, gap 16, sticky, hairline right border, `zs-card-flush` list | — | ✓ done last commit (verify vs mockup) |

- **A.i (pips data):** `(inbox)/+layout.ts` already has `tenantKey`+`accessToken`. Add a
  `getSegments(runId)` per listed run (batch the calls) → map to `KitRun.plan` via
  `segmentsToPlan`. Existing endpoint, existing table. No schema change.

### B. Right panel — `RunDetail` (mockup RunDetail)

| # | Element | Mockup | Existing source | Build action |
|---|---|---|---|---|
| B1 | header glyph | `観` lg, accent-when-asks | — | ✓ done |
| B2 | eyebrow | `Session · S-2891` | `run_id` (short) | ✓ done |
| B3 | title | run title | `run.title` | ✓ done |
| B4 | meta line | `repo · model · elapsed · edits · last activity` | model = `segments[].model` (graph); elapsed=`started_at`; last=`last_event_at`; repo §4; **edits dropped** | bind model from segments; drop edits |
| B5 | status chip | `running` | `run.status` | ✓ (verify tone) |
| B6 | progress | `Phase 3 of 5 · Implement` + bar | `segmentsToPlan`/`planProgress`; bar tone accent-when-asks | ✓ (bind tone) |
| B7 | tabs | `Needs you N` / `Plan` | gates count / — | ✓ done |
| B8 | **ask card** | `認 APPROVAL [blocking]` + prompt + context + `holds tN →` + **numbered options 1/2/3** + `type 1–N…` input + **Send answer** + hold-note | `relay_inbox.payload{prompt, options}`, `kind`, `gate_severity` | **restyle `RelayGateCard`** to the mockup AskCard: accent-soft header w/ kanji+APPROVAL/DECISION label+blocking chip, numbered options that fill the input, free-text + Send-answer, "run holds until you answer" line (keep the real `replyToGate`/offline-queue send) |
| B9 | Plan tab | ListSection card: Goal line + outline; selected task shows `state · agent · model · waits-on · spec_ref · summary` | `segments` (`segmentToTask` carries agent/model/spec) | render the per-task detail block (mockup RunDetail L527-545) |
| B10 | footer | `Activity` + `Conversation` grid | `segmentsToActivity`; chat = none | ✓ done (Activity real; Conversation stays the honest empty until chat is wired) |

---

## 4. The ONE field the existing flow does not deliver: repo/project name (A2, B4)

`dojo.relay_sessions` has no project column; `relay_segments` has none either. The daemon
DOES compute `project_slug` and sends it in the publish body, but the dōjō POST handler uses
it only for billing and discards it. So the repo name is not persisted in the dōjō.

**Recommendation (minimal, not a model "switch"):** persist the `project_slug` the daemon
already sends — one nullable column on `relay_sessions`, stored in the existing POST, returned
in the existing GET, mapped to `KitRun.project`. No daemon change (it already sends it), no
new federation. Until then, the row/meta omits the repo rather than duplicating the title.

---

## 5. Build plan (in order; verify each vs the rendered mockup before the next)

1. **Seed a real graph to verify against** — `sensei:plan` + `register_plan` for the sensei
   project; confirm it federates to `relay_segments`.
2. **Left rail** — A4 (kill title-dup in `toKitRun`), A6 (inbox fetches segments → pips),
   verify A1–A8 vs mockup.
3. **Ask card** — B8 restyle `RelayGateCard` to the AskCard form; verify vs mockup.
4. **Run-detail** — B4 model in meta, B6 tone, B9 per-task detail; verify B1–B10 vs mockup.
5. **Repo name** — §4 (persist `project_slug`) once approved.
6. Zero-errors gate (dojo check 0/0 + test) + field-by-field diff of the whole screen vs
   `~/Desktop/mockup.png` before calling done.
