# Project window · Overview — follow-ups (non-blocking)

Slot 4 shipped 2026-07-08: spec-doc-reviewer ✅ (2 rounds), done-gate-verifier all verifiable
gates PASS (partially-verified only for Tauri-visual), wrong-gate-hunter **clean** (0 tripping,
multi-repo verified live on documentation.wiki → "2 repos"). Real pane at
`(project)/project/[id]/overview/`. These are the deferred items.

## Cross-screen dependency (done-gate item ahead of the app)
- **Session rows navigate to `/project/{id}/sessions#{id}`, not the Replay pane** the spec's
  done gate expects. There is NO replay route in the app yet — `observatory-instruments-replay`
  is an unbuilt screen. The current target IS a real, working route (project sessions list,
  anchored to the session) — not a dead link. When the Replay screen ships, update the
  overview session-row `href` in `(project)/project/[id]/overview/+page.svelte` to deep-link
  into Replay scoped to the session id.

## Data-model deferrals (return 0 / not-inventing — correct handling)
- **`stats.memories.readyToShare` / `toMerge` are hardcoded 0.** `sensei.memory_status` enum
  (`proposed, active, reinforced, challenged, battle_tested, archived, rejected`) has no
  promotion-readiness or merge-readiness value. The backend returns 0 + documents the gap
  rather than invent a status. When `[[pipeline/memory]]` defines those statuses, wire the two
  sub-counts in `get_project_overview_stats` (pg_store.rs ~4133).
- **`top_recommendation.defaultAcp` is null on live data**, so the "send to {acp}" hero action
  never renders today; the recommendation rows carry no `default_acp`. Appears when the L2/L3
  generator populates it.

## Deviations accepted (recorded for Jerry)
- **Accept/Reject kept in the hero** in addition to the mockup's single send-to-acp action.
  Reason: Accept/Reject is the only UI feeding `MeasureVerdicts` and is covered by a live e2e
  (`project-window-flows.spec.ts` `rec-accept`/`rec-reject`). Dropping it for strict mockup
  fidelity would regress a tested pipeline. If strict fidelity is wanted, move the decision
  controls to the observatory insights triage (which already has them).
- **`recentSessions` wire carries `startedAt`/`completedAt` (ISO), not `duration`/`time`.** The
  client derives both display strings. Spec JSON shape was illustrative; source is consistent.

## Persona review (gate 4) — outcome
FIXED before commit (P0-B, real trust bug in new code): the warn dot read `open_drift` from
`get_quality_signals` (`open_drift_count`, `status != 'current'`) while the displayed
`stats.docDrift.open` used `status IN ('drifted','broken')` — divergent predicates, so the
warning could disagree with the shown number. Fixed: warn now reads `stats.docDrift.open` (one
source); `get_quality_signals` supplies only `ftr_7d`. `project_detail.rs` get_project_overview.
EVALUATED, NOT A BUG: P0-A (persona claimed `sessionTitle(null)` crashes) — the wire actually
sends `""` (empty string, type string), not null, so `"".trim() || 'session'` works; type is honest.

DEFERRED (persona enhancements/advisory, non-blocking):
- **`readyToShare`/`toMerge` "0 · 0" sub-line** — Analyst+UX want it suppressed when both 0. The
  count is accurate (0, feature unwired) so not a lie; suppress-when-zero is polish. One-liner in
  frontend `statBlocks()` when the promotion feature lands.
- **`get_project_overview_stats` uses `?` (500)** while sibling queries use `unwrap_or`. The client
  `getProjectOverview` falls back to all-quiet on 500, so no blank screen; a 500 is more observable
  than silent degrade. Left as-is (same reasoning as Slot 1's maturity query).
- **`list_folders_by_project` fetches all folders then filters kind in Rust** — perf: pulls nested
  dirs then discards ~99%. Can't change the shared method's SQL (other callers want all folders);
  add a scoped variant or a kind param. Local-daemon, low urgency.
- **Hero shows 3 buttons** when defaultAcp set (send-to-acp + Accept + Reject) — add a visual
  separator between the spec action and the Gap-1 decision controls.
- **docDrift "1541 of 170"** reads like a broken fraction (open counts items, referencedDocs counts
  docs; items>docs is expected). A label like "(across 170 docs)" would clarify.

## Minor / debt
- **Duration + relative-time formatting now in 3 places** (`RecentSessions.svelte`,
  `buckets.lastSessionLabel`, `overview-view.svelte.ts`). Absorb into a shared `src/lib/time.ts`.
- **Legacy dead route** `(observatory)/projects/[id]/+page.svelte` (old TabBar, unreachable —
  `openProjectWindow` goes to `/project/{id}`). Candidate for deletion.
- **Scanner data quality**: `documentation.wiki` has two folders both named "documentation.wiki",
  making its multi-repo chips indistinguishable. Scanner concern, not this screen.
