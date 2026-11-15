# Checkpoint — GitHub tracker adapter (2026-11-09)

**Slice:** GitHub Project (v2) as a tracker, sprints from its Iteration field, GraphQL reads,
a GitHub token (D92, amending D71, D91, FT3, FT5, O5, O6). Slice 2 of the forge trackers.
Branch `feature/forge-trackers`.

**Stage:** design done, **gate PASSED**. Spec `docs/spec/daemon/tracker-github.md` r2.3.
Plan `docs/plans/2026-11-09-tracker-github.md` (G1–G6).

**Why Projects, not milestones:** sensei-hq/sensei has no milestones (C16); the user plans
sprints in a GitHub Project (2026-11-09). **Why a token:** the forge sign-in lacks
`read:project`, and adding it changes the dōjō login for every user.

**Claims:** 28 ledgered, 0 FALSE, 4 MISLEADING, each changed the design:
- C3: `query_json` under `impl<R: RawGet>` would break RawGet-only test backends → `RawGet::post_graphql` default `Err`, bound kept.
- C11: `ValidateError` is `Copy`, relied on at `gitlab.rs:409-417` → drop `Copy`, fix gitlab.rs.
- C15: `errorFor` returns early for `board_not_found` → a GitHub branch there, not a words map.
- C6: `seq` has six readers, all ordering.

**Review (6 reviewers + 3 re-checks, none NOT RUN):** biggest catches: a bare `#12` lost its
host so GitLab could answer for GitHub (H5 keeps the host); the GraphQL guard and a
NOT_FOUND oracle (security, signed off on r2.1); D71 had to be amended; fixture scrub by
allowlist with a structural test.

**Build (2026-11-09, unattended):**
- Fixture projects created in sensei-hq: #2 "sensei tracker fixture" (Sprint field: A completed,
  B current, C future; issues from sensei and dbd, a PR, a draft, an archived item) and #3 (no
  Iteration field). The user's real board is #1 (Iteration field, 100 sensei issues), for G6.
- G1–G3 8ea43ad30 (+ docs e7e29afb8), G4–G5 747c1cffc (+ docs 9289061f5).
- Build amendments: `ConnectParams.sprint_field`, `BoardScope.params`; `normalize_repo_key` at
  its real path (ratchet); the recorder's iteration-title scrub fixed after a test caught it.
- Gates: senseid 4435, cli 92, app 3206 (2 pre-existing failures), clippy and svelte-check clean.
  Adapter tests were written after the draft; two mutations (type filter, containment) each
  turned tests red.

**Whole-slice review (2026-11-09):** security NO FINDINGS; failure-mode, test and adversarial
findings fixed red-first in f1ab509c4 (+ docs cda9abc2d). User decision: GitHub tokens are per
project owner (`params.credential_scope`), narrowing the linkage spec's D14 for GitHub. Gates:
senseid 4448, cli 92, app 3206 (2 pre-existing), clippy and svelte-check clean.

**G6 live (2026-11-09), passed:** this branch's debug daemon; only the `github` enum hook applied
to the live DB (a backup was taken first; `dbd diff` also shows an unrelated
`activity.assistant_events` generated-column drift, left alone). The sensei workspace connected to
github.com/orgs/sensei-hq/projects/1: 5 sprints (Iteration 1 active, 2–5 future, start + 14 days)
and 113 tickets, equal to the project's 113 non-archived issue items (none has an iteration yet,
so all are Backlog). `get_ticket` `#39` (bare and full) → the issue; a PR number and a missing
number → the same 404. Ten existing sessions linked by bare `#n` in their prompts.

**Next:** close #371–#376 (and the GitLab #364–#369) when the user says so; PR to main is the
user's call.
