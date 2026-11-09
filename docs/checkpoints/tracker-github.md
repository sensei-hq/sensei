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

**Next:** P1 (human): `gh auth refresh -s project`. Then `/sensei:build` from G1 (G1 needs no
fixture and can start now).
