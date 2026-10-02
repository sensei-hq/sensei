# Checkpoint

**Slice:** Observatory diagrams — data (indexer) → derivation (views) → screens.
Lane 2 (derivation) is complete: a project resolves ONE way.

## Done

- **#211 CLOSED — `folders.project_id` dropped** (17721622). Membership is the
  repository's: `folders.repository_id` → `project_repositories` → `projects`,
  via `sensei.folder_projects`. Dropped rather than backfilled because 601
  folders disagreed with the junction in one REPEATABLE READ snapshot, zero an
  hour earlier — the column was a second WRITER, not stale data.
  `sensei.sole_project_of()` holds the sole-member rule (NULL for none-or-two;
  9,595 sole, 4,129 not).
- **Three live defects the drop exposed**, all fixed in that commit:
  1. Governance leaked across projects. `resolve_rules_raw` scoped by
     `m.project_id = (SELECT project_id FROM sensei.folders …)`; with the column
     gone the bare name bound OUTWARD, so the predicate read
     `m.project_id = m.project_id`. One real folder got 9 rules from 8 projects
     where the answer is its own 2. Now an EXISTS over `folder_projects`.
  2. `heal_nested_standalone_roots` aborted on the first project-less repo —
     nullable `sole_project_of` decoded as a bare `Uuid`, so NOTHING anywhere
     was re-absorbed.
  3. `upsert_folder` kept the `project_id` bind after dropping it from the
     column list, so `$7` put a project uuid in `workspace_root_id`.
- **#205, #206 closed** earlier in the slice (Structure endpoint + screen;
  26,461 ms → 863 ms).

## Next

    cargo test -p senseid --bins   # baseline before the next screen

Then lane 3, next screen in the Observatory diagram set. Pick it from
`docs/backlog.md`; the Structure screen is the worked template
(`docs/spec/screen/`, handler + `*-state.svelte.ts` + spec + e2e).

## Open questions

- 231 uncommented columns filed as an issue; not blocking.
- #213/#214 and #201–#204 still open.

## Known broken

- Nothing known broken. senseid 3196/0, rest of workspace 361/0,
  `clippy -D warnings` and `fmt --check` clean. 30+ commits UNPUSHED on
  `develop` (nothing has gone to the remote since the history scrub).
