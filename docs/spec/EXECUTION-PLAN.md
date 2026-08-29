# Sensei — 5-day vacation execution plan

**Locked:** 2026-07-07
**Owner during vacation:** autonomous, gated per doc
**Location:** committed here so a phone session picks it up cold
**Superseded when:** you return + review; overrides via a fresh commit

---

## Ground rules

1. **Spec-first, gated per doc.** Loop per doc:
   `spec-doc-reviewer → implement → done-gate-verifier +
   wrong-gate-hunter (parallel) → sensei-persona-reviewer →
   commit`. See [`agents/README.md`](agents/README.md).
2. **3-try-then-park.** If a gate fails, try three root-cause
   attempts. If still failing, park at
   `docs/spec/park/{doc}.md` with the gate output + best
   guess + explicit `AWAITS: Jerry` line. Continue to next doc.
3. **Real data.** Point sensei at `~/Developer` + `~/Work`. No
   dummy corpus. Wipe seed roots if any remain; run against live.
4. **Commit rhythm.** One commit per doc when all gates pass.
   Push develop after each commit (safe, gated). Never merge
   develop → main autonomously — Jerry does that on return.
5. **Voice.** No filler ("honest", "honestly", "truly"). No
   marketing terms. Sentence case. Lowercase `sensei` /
   `ollama`. See
   (memory: feedback_no_filler_honest).

---

## What "done end-to-end" means for this run

Realistic target: **6–8 shipped-and-verified screens** across
segments 03 (Observatory) + 04 (Project window). Depth over
breadth. The value is in the verification depth (spec → code →
gates all green → sensei-persona-reviewer approves), not surface
coverage.

Non-targets:
- Dōjō SaaS console screens (require external infra).
- Solution-scope screens (no mockup yet — greenfield).
- New pipelines that touch DDL (add rows only if the target
  screen needs them).

---

## The queue — 6 target screens in dependency order

Ordered so each screen's data invariants are already true when
we start. Every screen references its own spec doc for the
five-section contract.

### Slot 1 — Observatory · Today (flagship)

**Spec:** [`screen/observatory-today.md`](screen/observatory-today.md)

**Why first:**
- The user's landing surface — highest-visibility.
- Depends on FTR ([[pipeline/ftr]] ✅ live) + insights ([[pipeline/insights]] partial) + narration-cache ([[pipeline/narration-cache]] deferred: static fallback OK for now) + memory ([[pipeline/memory]] adopted lane).
- Data plumbing mostly exists; the win is UI + wire-up.

**Data-invariants prep needed:**
- `GET /api/observatory/today` — verify handler exists;
  if not, wire from existing pieces.
- `dataMaturity` field — server-side decision; add if missing.

**Gate checklist:**
- spec-doc-reviewer
- implement
- done-gate-verifier (curl checks on `/api/observatory/today`)
- wrong-gate-hunter (koan copy uniqueness; adopted lane
  scope; FTR chip match with Projects card)
- sensei-persona-reviewer

**Definition of shipped:** early / mature variant renders on real
data; hero koan populated (fallback text OK if gemma4 not
running); FTR chip agrees with Projects screen; adopted lane
reflects real memory state; recent-sessions row navigates to
Replay.

---

### Slot 2 — Observatory · Instruments · Health

**Spec:** [`screen/observatory-instruments-health.md`](screen/observatory-instruments-health.md)

**Why:**
- Signal derivation shipped 2026-07-07 in `tool_signals.rs` —
  code side already live on develop.
- The mockup pivot to L1 MCP grid + L2 per-tool is new UI work.
- L1 grid needs `share_invoked = tools_invoked_14d /
  tools_registered` — small DB view.

**Data-invariants prep needed:**
- `GET /api/observatory/mcp-servers` — add if missing.
- MCP grid rendering — new component.

**Gate checklist:**
- spec-doc-reviewer
- implement (mostly UI: L1 grid + drill to L2; L2 already works)
- done-gate-verifier
- wrong-gate-hunter (share bar accuracy; disconnected
  MCP behaviour; subNav placement below hero)
- sensei-persona-reviewer

**Definition of shipped:** L1 renders one card per connected
MCP with correct share bar; clicking drills into per-MCP L2;
signals collapse rules honored.

---

### Slot 3 — Observatory · Projects (list-view addition)

**Spec:** [`screen/observatory-projects.md`](screen/observatory-projects.md)

**Why:**
- Rebuild landed 2026-07-07 — grid view live.
- Mockup added a list-view toggle (`≣`) — small addition on top.
- Depends on project-icon pipeline for image icons (deferred if
  scope creeps).

**Data-invariants prep needed:**
- View preference persistence (localStorage OR a settings row).
- List-view Svelte component (ProjectRow analog).

**Gate checklist:**
- spec-doc-reviewer
- implement (grid → grid+list toggle)
- done-gate-verifier
- wrong-gate-hunter (view toggle regression; row wrap; vision
  truncation)
- sensei-persona-reviewer

**Definition of shipped:** grid ↔ list toggle persists; row
layout matches mockup; card and row render the same data
consistently.

---

### Slot 4 — Project window · Overview

**Spec:** [`screen/project-overview.md`](screen/project-overview.md)

**Why:**
- Landing pane inside a project window.
- Depends on FTR ✅ + top recommendation ([[pipeline/insights]]
  partial) + memory counts ✅ + doc-drift ([[pipeline/traceability]]
  partial).
- Multi-repo membership rendering is new (folder-role chip on
  sessions).

**Data-invariants prep needed:**
- `GET /api/projects/{id}/overview` — assemble from existing
  pieces if not present.
- Vision field on `sensei.projects` — new column, small
  migration.

**Gate checklist:**
- spec-doc-reviewer
- implement
- done-gate-verifier
- wrong-gate-hunter (multi-repo folder-role rendering; hero
  early-state text; FTR chip match; row session-id resolution)
- sensei-persona-reviewer

**Definition of shipped:** hero + stats + recent sessions + FTR
all render on real data for the sensei project; multi-repo
projects show the folder-role chips.

---

### Slot 5 — Observatory · Insights (Learnings Triage)

**Spec:** [`screen/observatory-insights.md`](screen/observatory-insights.md)

**Why:**
- The triage surface for the recommendation pipeline.
- Depends on [[pipeline/insights]] generator — partially live;
  buckets query needs finishing.
- Verb set (Apply · Review · Dismiss) needs the ergonomics
  worked out.

**Data-invariants prep needed:**
- `GET /api/insights` — server-side bucketing (Now / Soon /
  Settled).
- `POST` endpoints for apply / review / dismiss actions.

**Gate checklist:**
- spec-doc-reviewer
- implement
- done-gate-verifier
- wrong-gate-hunter (bucket rules divergence; MeasureVerdicts
  scheduling on apply; verb-highlight consistency)
- sensei-persona-reviewer

**Definition of shipped:** three columns render with real data;
apply schedules a `MeasureVerdicts` follow-up; verbs are
one-decision-one-default consistent.

---

### Slot 6 — Observatory · Sessions

**Spec:** [`screen/observatory-sessions.md`](screen/observatory-sessions.md)

**Why:**
- Compact digest of activity — high daily-use value.
- The chart-variant work is new (trend / stream / constellation /
  bands / pulse).
- Session-id resolution regression already caught + fixed once
  — must survive.

**Data-invariants prep needed:**
- `GET /api/sessions?range=…&project=…` — mostly present.
- Chart-variant components — new work.

**Gate checklist:**
- spec-doc-reviewer
- implement
- done-gate-verifier
- wrong-gate-hunter (session-id resolution; FTR badge match
  with Replay; chart-variant coverage)
- sensei-persona-reviewer

**Definition of shipped:** default trend variant renders; at
least one alternate variant works; row click into Replay
resolves session-id correctly.

---

## Overflow — slot 7-8 if time allows

Ordered by leverage:

7. **Observatory · Memories** (Learnings Anatomy) — the LLM-primary
   consumer curation surface. Depends on [[pipeline/memory]]
   promotion ladder wire-up + telemetry rendering.
8. **Project window · Sessions + Memories** — the project-scoped
   variants; small delta over their Observatory peers now that
   the primitives are done.

If neither ships, that's fine — 6 done end-to-end beats 8 done
half-way.

---

## Off-limits (until Jerry approves)

- Any DDL change that isn't strictly required by a target screen.
- Any change to `main` branch (autonomous runs stay on develop).
- Anything touching `.env`, credentials, or the daemon's data
  directory.
- Adding new pipelines beyond what a target screen requires.
- Deploying / publishing / running `make bump`.
- Making purchases against external providers (openai / anthropic
  / etc.); everything routes to embedded ollama.

## Escalations

Park a doc when:
- 3 root-cause attempts still fail.
- The daemon is unreachable and can't be recovered locally.
- A gate agent's advice contradicts a spec's design intent.
- The mockup and the spec disagree and neither is obviously
  authoritative.
- A change would require a DDL migration bigger than adding one
  column or a small view.

## Daily rhythm

- **Morning tick (autonomous):** advance the next queued slot;
  commit + push per doc.
- **Every 6 hours:** run the wrong-gate-hunter over the last 3
  merged specs to catch regressions.
- **Nightly:** persist the run's state to `docs/spec/park/`
  as a summary note so a phone-session pickup sees the delta.

## When Jerry gets back

Merge sequence Jerry does manually:
1. `git fetch --all` and read the develop log.
2. Cherry-pick the parked docs; decide accept / redirect for
   each.
3. If ≥ 4 slots shipped clean and gates all pass, merge develop
   → main and bump.

---

## Related

- [`README.md`](README.md) — the spec system + vision
- [`agents/README.md`](agents/README.md) — the gate playbook
- [`MOCKUP-INDEX.md`](MOCKUP-INDEX.md) — mockup lookup
- (memory: project_vacation_run_2026_07) — the vacation-run
  memory note this file mirrors
