# Master backlog — full spec build (Jerry: "hit everything, incl. Dōjō")

Ordered work queue for the autonomous run. Process top-down in phase order; each item runs the
gated loop (spec-doc-reviewer → implement → done-gate + wrong-gate → persona → commit); merge+bump
at phase milestones. Mark `[x]` + commit hash when shipped, `[~]` building, `[park]` if blocked
(default-and-proceed: park only irreversible/external). Re-entry: read `_run-state.md` (policies +
position) then this file (what's left).

## ✅ Phase 0 — original 6-slot queue (DONE)
- [x] observatory-today `35a438ce`
- [x] observatory-projects `ead8f971`
- [x] project-overview `fa18a4d1`
- [x] observatory-insights `035a368c`
- [x] observatory-sessions `a83303c6`
- [x] observatory-instruments-health `6336dc6a` — un-parked + SHIPPED (full-capture tool-health grid; all gates + persona)

## Phase 1 — finish Slot 2 ✅ DONE
- [x] observatory-instruments-health `6336dc6a` (assistant_tools inventory + per-assistant ToolDiscovery trait + full capture + real 14d grid + L1 UI). MILESTONE: merge develop→main + bump v0.2.24 IN PROGRESS.

## Phase 2 — DEPTH: make the 5 shipped screens fully real (burn down deferrals = build the pipeline gaps)
- [ ] pipeline/narration-cache — wire gemma4 copy chain; replace raw-DB-text fallback on Today/Insights/Projects/Overview cards
- [ ] pipeline/memory — define promotion/merge-readiness statuses; wire readyToShare/toMerge (Overview, Insights); adopted-lane
- [ ] pipeline/insights + pipeline/patterns + pipeline/signals — the recommendation/pattern generators (tables exist, writers missing)
- [ ] per-screen followups: observatory-today, observatory-projects (all-view chip, perf), project-overview (Replay nav, list_folders perf), observatory-insights (show-N-of-M, ViolationCard nav), observatory-sessions (all chip, URL range) — see park/*-followups.md

## Phase 3 — overflow (original 7/8)
- [ ] observatory-memories (Learnings Anatomy)
- [ ] project-sessions ; project-memories

## Phase 4 — BREADTH: remaining Observatory + Project + entry screens
- Observatory: [ ] observatory-impact [ ] observatory-libraries [ ] observatory-logs [ ] observatory-traceability [ ] observatory-upgrades [ ] observatory-consolidation [ ] observatory-instruments-playground [ ] observatory-instruments-replay [ ] insights-reasoning
- Project window: [ ] project-about [ ] project-impact [ ] project-instruments [ ] project-libraries [ ] project-patterns [ ] project-traceability
- Entry/setup: [ ] bootstrap-green [ ] bootstrap-probing [ ] first-run-scan [ ] first-entry-projects [ ] preferences [ ] settings-inference
- Solution scope: [ ] solution-dashboard [ ] solution-architecture [ ] solution-sessions
- Supporting pipelines as needed: analyzer, ftr, impact, traceability, libraries, library-intelligence, project-icon, semantic-search, testability, context-delivery, agent-execution, clarification-prompting, benchmarks, bootstrap-resolution, capture, mcp-surface, inferencing

## Phase 5 — DŌJŌ segment (last — needs auth infra)
- Auth infra FIRST: Supabase (assume localhost URL) + kavach (`~/Developer/kavach`, has supabase/ dir — EDIT if needed). Localhost Dōjō registry URL.
- Pipelines: [ ] pipeline/collective-intelligence [ ] pipeline/dojo-lifecycle [ ] pipeline/governance
- Observatory Dōjō: [ ] observatory-collective [ ] observatory-dojo-connections [ ] observatory-dojo-sharing [ ] observatory-share-review
- Dōjō consoles (SaaS): [ ] dojo-developer-flow [ ] dojo-lead-console [ ] dojo-maintainer-console [ ] dojo-admin-console

## Notes
- Some screens/pipelines may already be partially live (memory notes: analyzer/ftr/scheduler shipped; libraries/traceability/impact partial). ASSESS each at pickup (grep + curl) before building — don't rebuild what exists; wire/finish it.
- Merge+bump milestones: after Phase 1, after Phase 2, after Phase 3, then per-phase.
