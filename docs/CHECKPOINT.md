# Checkpoint — 2026-11-14 (forge comment write-back: W1–W7 built on feature/forge-comment-writeback)

**Slice:** comment write-back for GitLab and GitHub (spec
`docs/spec/daemon/tracker-forge-comment-writeback.md`, plan
`docs/plans/2026-11-14-forge-comment-writeback.md`, issues #436–#443, D93). Built unattended
(user, 2026-11-14).

**Done (pushed):** W1 `7305d36c3`, W3 `cb0d3dfb7`, W4 `fcad91d78`, W2+W5+W6 `867687b79`,
W7 `de85312c6`. Each feature had red-first or mutation-checked tests, fmt and clippy
(`-D warnings`) clean, and an adversarial review whose CRITICAL/HIGH/MEDIUM findings were
fixed. Full senseid suite: 4902 passed (before the W5 review fixes; tracker and handler
tests 409 after). App: 172 unit tests, svelte-check clean. CLI: 100 tests.

**Amended at build (all recorded in the spec and `.sensei/slice-state.json`):**
- `Patch` and W2 moved into W5; the forge visibility reads moved into W4 (clippy dead code).
- The visibility check runs after `writeback_target` returns Go.
- A failed visibility read is final only on 404.
- A rate-limit 403 is `rate_limited` on reads too; 410 → `NotFound`.
- `issue_exists` checks identity.
- The sanitiser defuses line-start blocks.
- GitLab keys need a namespace.
- `github::is_repo_name` refuses `.` and `..`.

**Next command:** open a PR `feature/forge-comment-writeback` → `develop`. After merge, W8
(#443, the user's): the live check X2, X3, X5, X6 on gitlab.com and github.com test projects.
Then `/sensei:complete`, which closes #436–#443.

**Open questions (user):** none.


---

# Checkpoint — 2026-11-16 (relay follow-ups 2: PR #460 open, held for the v0.23.0 release decision)

**Slice:** spec-first relay follow-ups on `lane/relay-followups` (`openspec/changes/`).

**Done:** #456 merged (#450 #451 closed). PR #460 → develop: #422 Lanes by run / by agent (daemon `run_tasks`,
`toRunGroups`, kept per viewer), #425 e2e, #396 lane host (`relay.lanes.host`, register/guidance/MCP/command, "in
Cursor"), #449 the journey's seeded ticket, README. Relay e2e green on the installed daemon; live `sensei` and
`sensei_e2e` reconciled (two passes: dbd#117). Issues #457 (docs → wiki), #458 (adopt OpenSpec).
**#446** parked on `wip/446-sqlite-cve`: rusqlite 0.43 + dbd-core without sqlite clears RUSTSEC-2026-0459
(`cargo audit` red → green shown), but sqlx's default features pull sqlx-sqlite → fixed in sensei-hq/dbd#118; the
`dbd_pins` guard needs a dbd **tag** (a tag publishes to crates.io).

**Next command:** on the user's word — merge #460 (now, joining release PR #459, or after it); merge dbd#118 + release
dbd v0.27.10, then move all four dbd pins and land `wip/446-sqlite-cve`; then #447 (labelled set + baseline).

**Open questions (user):** #460 into v0.23.0 or after; the dbd release; #447 floors after the baseline; #457.
