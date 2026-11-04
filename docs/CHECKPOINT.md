# Checkpoint — 2026-11-04 (coordinator; lanes in ~/Developer/sensei-hq/coord/TRACKER.md)

**Slice:** the week's spine (orientation §3): sync verified → targets end to end → a miss becomes a fix and a
learning. Five implementation queues + Cowork design (plan "Queue v3"); reviews through `~/Developer/sensei-hq/coord`.

**On develop (2026-11-03 → 04):**
- Decisions D47–D86; PRD, journeys and roadmap 01–09 aligned and checked against code.
- Guards in `make test-fast` + CI: no fabricated values (D83 ratchet), doc links, shared JS specs, tracker docs.
- Metrics that act: ActionItems (workspace isolation on every recommendation handler), D83 health/correlations.
- S7 focus, S4 2c (one reading per repository/person/day), TG1 metric gating, Copilot unknown ≠ 0.
- Dependencies: one version per shared dep, `[workspace.dependencies]`, tauri 2.12, ureq 3; kavach 1.3.2.
- PR #350 (pavan-katakam): SDLC metrics + tracker write-back, off by default (D50). Its own status entry
  ("complete, partial; done gate run live") is at 3f7e22ad3:docs/CHECKPOINT.md.

**Remaining, in order:**
1. L1 F4 r4 → `run.sh --apply` → WP8 → WP9A → D18 label → #314 production sync (critical path; TG2 waits on WP8).
2. #156 r2 (fix `folder_completeness`: every live repo reads incomplete) → TG2–TG5 → targets UI → D66 learning.
3. Notice r3 (Cowork) → L5 libraries merge; menu (D70) and billing mockups.
4. **Install** (nothing merged since 28 Oct is live): converge the live DB twice (ActionItems `snoozed_until`,
   TG1 both planes, F4 CHECK, SDLC tables), `make install` with the user (Keychain), live checks, F4 re-push.

**Next command:** review/merge as lanes report; then the install checklist in coord/TRACKER.md "Deploys".

**Open:** one allowlist rule applied two ways: PR #350 allows `coalesce(modified_at, now())` in seed imports,
while TG1 made the same pattern raise. `repository_metric_series.sql:336` flakes across the UTC day (→ L1).

**Known broken:** the installed daemon (28 Oct) fails the e2e diagrams-schema test until the install.
