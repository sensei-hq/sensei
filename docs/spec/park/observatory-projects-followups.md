# Observatory · Projects — follow-ups (non-blocking)

Slot 3 shipped 2026-07-07: spec-doc-reviewer ✅ (re-review ready-to-implement),
done-gate-verifier **ready-to-ship**, wrong-gate-hunter **clean** (all 11 absent).
These flags came out of the gates; none block. All pre-existing or harmless.

## Type/shape
- **`stack` shape mismatch (list vs detail).** The extended `/api/projects` list endpoint
  returns `stack` as the raw jsonb array (`["python"]`), but `ProjectListItem.stack` is typed
  as `{ languages?: string[]; … }` (the DETAIL-endpoint shape used by 4 consumers:
  `projects/[id]`, `project/[id]/about`, InstrumentsSection, ProjectsSection). Harmless on the
  Projects screen — the refactored card/row dropped the stack stat, so `stack` is never read
  here. Fix: either drop `stack` from `list_projects` (it's vestigial after the card refactor —
  the spec's card no longer shows it), or make the type a union. Chose not to churn a build
  cycle for an unread field. `bun run check` passes (no consumer accesses list `stack`).

## Consistency
- **Two repo-count queries use different kind filters.** `list_projects` uses
  `kind IN ('git','standalone')`; `get_project_repos` (Project › Overview) uses
  `kind <> 'folder'`. Equivalent today (only 3 kinds exist: folder/git/standalone) so values
  match, but they'd diverge silently if a new folder kind is introduced. Align both to one
  filter. Also: `libs_count` shows on the Projects card but Project › Overview has no libs stat
  (structural omission, not a mismatch).

## Performance
- **N+1 + heavy fanout on the Projects list.** `list_solutions` (observatory.rs) calls
  `list_folders_by_project` once per project (298 round trips for 297 projects) to attach a
  `folders[]` array — needed by Settings › Projects setup, now redundant for the Projects
  screen (which uses `repos_count`). Plus the frontend fans out `/ftr` + `/quality-signals`
  per project. Pre-existing (my change added only correlated subqueries to the one main query,
  not round trips). Consider: gate the `folders[]` enrichment behind a query param so the list
  load doesn't pay for it, and/or a batched ftr/quality endpoint. Not a Slot-3 regression.

## Persona review (gate 4) — outcome
FIXED before commit (P0, was a real bug in new code): `isWarning` fired for all 292 dormant
projects because the quality-signals wire fallback is `ftr_7d: 0` and `0 < 0.6` = true → every
dormant project showed an amber warning dot, destroying the signal at scale. Fix: `isWarning`
now takes `sessions7d` and returns false when `sessions7d <= 0`; and `+page.ts` only fans out
ftr/quality for ACTIVE projects (uses the new `p.sessions7d` from the list endpoint for the
active/dormant decision). Bonus: collapses the load fanout from 2×297≈594 requests to 2×(active
few ≈5). Verified: check 0/0, 745 tests (+1 dormant-guard test).

DEFERRED (persona recommendations, non-blocking, some deviate from spec):
- **Adaptive default view** — persona wants list (not grid) as the default when no stored pref
  and count > 100. The spec explicitly says "Grid — the default", so this deviates; leave for a
  Jerry design call. (localStorage still makes the choice sticky after first toggle.)
- **Dormant status pill is omnipresent** at this scale (292/297 rows carry it) — reads as
  furniture. Consider suppressing the pill in "All" view when it's not distinguishing.
- **Recency sort for dormant** — dormant sorts alphabetically; a `last_session_at` desc option
  would help find "the project I paused last week" among 292.
- **Minor (Developer)**: stat-grid wrapper `<div>` duplicated across active/dormant variants in
  ProjectCard (extract); ProjectRow lacks a warn-tone test for the active ftr span; filter/query
  are transient while view persists (asymmetric — arguably correct).

## Not-verifiable-here (manual, in the Tauri app)
- Card/row click opens the project window; dark-mode readability; real-time search feel.
  `onOpen(p.id, p.name)` is wired in both components + the Tauri capability manifest includes
  `project-*` windows + `core:webview:allow-create-webview-window` — verify visually on return.
