# Observatory · Insights (Learnings Triage) — follow-ups (non-blocking)

Slot 5 shipped 2026-07-08: spec-doc-reviewer ✅ (2 rounds), done-gate-verifier 6/7 → after fix
all pass, wrong-gate-hunter clean after the same fix. Screen at `(observatory)/insights/`.

## FIXED before commit (was the single blocking issue on BOTH mechanical gates)
- **Apply did not schedule MeasureVerdicts.** The reused `/accept` handler
  (`accept_project_recommendation`, project_detail.rs) set `status='accepted'` and returned —
  MeasureVerdicts was only enqueued by the periodic scheduler, so the spec's "Apply schedules a
  MeasureVerdicts follow-up" FTR feedback loop was open. FIXED: the handler now enqueues
  `Task::new(TaskKind::MeasureVerdicts, "", "")` after a successful accept. Also improves the
  Slot-4 project-window Accept button (same handler).

## Deferred (documented; none block)
- **narration-cache not wired in `get_insights`.** Rec `title`/`why`/`impact` come straight from
  `inference.recommendations` columns — raw DB text, not routed through `[[pipeline/narration-cache]]`.
  Per the run-wide decision narration-cache is deferred and static/raw text is the accepted fallback;
  the wrong-gate symptom (homogeneous copy) is ABSENT (live titles are varied). Wire the copy chain
  when narration-cache lands (#65).
- **Server `counts` in the /api/insights payload is unused by the client.** The frontend recomputes
  counts from the live arrays so optimistic Apply/Dismiss removals stay honest (a card removed on
  Apply must decrement its column). The server counts are therefore dead payload — harmless, but
  could be dropped or documented as "initial only". (The client behaviour is correct.)
- **Violations in the Now column have no action surface.** ViolationCard is display-only (memory
  write-actions reinforce/challenge/archive are deferred — only `/promote` exists). The spec calls
  the Now column "where the day's decisions live", but a violation's decision (reinforce/challenge)
  has no button here. Wire the memory transitions when those endpoints exist.
- **Memory write-actions (reinforce/challenge/archive) endpoints don't exist** — only
  `/api/knowledge/memories/{id}/promote`. Build them to make memory cards actionable on this screen.

## Persona review (gate 4) — outcome
- **`LIMIT 200` silent truncation (was a "no silent caps" gap in new code) — MITIGATED.** With 337
  pending recs, the board shows the top-200 by urgency and silently dropped ~137 lower-urgency ones.
  Added a `tracing::warn!` when the cap is hit (get_insights_recommendations, pg_store.rs) so it's
  observable server-side. FULL fix (follow-up): return per-column TRUE totals + a "showing N of M"
  UI hint + "show more". The top-200-by-urgency behaviour is otherwise defensible triage.
- **ViolationCard is a dead-end (persona P0)** — spec-COMPLIANT (violation/memory write-actions are
  deferred; done-gate-verifier passed display-only cards), so NOT a blocker, but a real UX gap:
  the highest-signal Now card has no exit. `ViolationCardVm.projectId` + `openProjectWindow` already
  exist — a nav-only "Review" verb is ~5 lines and consistent with the deferred-write constraint.
  HIGH-PRIORITY enhancement follow-up.
- **`board.loading` set but never read** in `+page.svelte` — wire a spinner/opacity on the
  project-filter refetch (immediate-feedback rule). Follow-up.
- **Soon column at 184 items** is a second inbox — client sub-cap to ~25 with "show more". Follow-up.
- **`aria-label` on the three triage `<section>`s** for screen-reader orientation. Follow-up.
- **Corrections carry no `project_id`** → excluded from the project-filter chip `seen` set. Documented
  constraint (corrections are cross-project aggregates) — confirm intended.
- **`strength` clamp 0–1** assumes the DB stores a 0–1 fraction; verify the `sensei.memories.strength`
  scale so the strength bar never mis-fills.

## Deviation accepted
- Dropped the mockup's typed-descriptor kind-chip (glyph/tag) on rec cards — the wire carries no
  `action_type`, so the card shows project chip + impact sentence + why instead (WIRE wins over
  mockup). Restore the kind-chip if the recommendations pipeline starts emitting an action type.
