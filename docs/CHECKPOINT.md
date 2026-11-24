# Checkpoint — 2026-11-23 (#268 landed; #246 discussion next)

**Slice:** relay + diagrams open issues.
**Branch:** `develop` (pushed: 4 #444 + 6 #268 commits).

## Done
- Splits filed: #427 → #522–#526; #428 → #527–#530.
  New: #531 sort, #532/#533 repo/workspace screens, #534 muse.
- #299 verified done as workspaces (default 1/repo, multi
  capable) — ready to close. #312/#433 still await close confirm.
- #444: janitor, sentinel, serial guard, execute endpoint.
- #268: dojo graph reads, CodeMapPort (diffed vs daemon),
  repo diagrams route + picker + lenses, branch manager,
  schema ER lens. Dojo 2527, DB 83 files, build green.

## Remaining
- Push the #268 commits (task 6 tick).
- REMIND USER: discuss #246 (docs ring design).
- User owes: mobile layout specifics for a new issue.

## Next command
`git add openspec/changes/add-dojo-diagrams/tasks.md docs/CHECKPOINT.md && git commit && git push origin develop`.

## Open questions
- 6 process.rs fmt spots: local toolchains flag what CI stable
  accepts. Left untouched.
- Package `ZonePoint` type lies about nulls (daemon sends
  them) — follow-up issue proposed, not filed.

## Known-broken
- None.
