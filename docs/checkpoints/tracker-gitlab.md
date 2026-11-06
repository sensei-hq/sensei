# Checkpoint — GitLab tracker adapter (2026-11-06)

**Slice:** GitLab as a tracker (gitlab.com + self-managed, Free/CE up), daemon path. Slice 1 of
`docs/analysis/2026-11-06-forge-trackers.md`; GitHub is slice 2. Branch `feature/forge-trackers`.

**Stage:** design done, gate PASSED. Spec `docs/spec/daemon/tracker-gitlab.md` (r3, commit e345df047; r3's §G5 change was audited by `sensei-spec-conformance-auditor` at build start, 6 findings fixed).
Decision D91 (amends D49).

**Claims found false, and what each changed:**
- C9: sync already merges `params` → r2 extends the existing patch instead of adding a second merge path.
- C16: 5 transport sites, not 4, and history/write-back bypass `transport_for` → r3 puts the address check inside `ReqwestRaw`, which both factory methods build (C34).
- C25: `points_field = None` means "not asked" → GitLab v1 reports no points (`unsupported_by_tracker`).
- r1 "No sprints yet" copy, the 400 status, the CLI prompt → corrected.
- Not ledgered in r1: `is_public_ip` already existed (`indexer/lib_indexer.rs:250`) → extracted to `net/addr.rs` (DRY).

**Decided (user, 2026-11-06):** OPEN-G1 and OPEN-G2: no ports, subpaths or private networks. The opt-in and `NetPolicy` are removed (spec r3).
**Plan:** `docs/plans/2026-11-06-tracker-gitlab.md`, F1–F6 = issues #364–#369.
**Human prerequisites for build:** recorded fixtures from a gitlab.com Free group and a Premium trial group.

**Build (2026-11-06, unattended):**
- F1 #364 done: 881558907 (public-only resolver, no proxy), 3a2bfb80a (header auth, trailing dot),
  2e43fdac0 (validated board_ref/account, params_extra), 34d1351d8 (capabilities + guards,
  `unsupported_by_tracker`), c542f9a28 (review fixes). Full senseid suite 4367 passed at 34d1351d8.
  Review: security NO FINDINGS; test reviewer's 4 gaps fixed; adversarial: 1 fixed, 1 LOW left
  (`reads_history` re-reads the tracker row).
- F2 #365 done: 40f8984cc (`tracker::forge`, `link::tally_links` shared with Jira).
- Two `#[allow(dead_code)]` wait for F3 to remove: `AuthScheme::Header`, `mod forge`.
- Amended: D10 end to end (HTTP 409, SDLC reasons through the DB) moves to F3.

**Blocked on the human:** P1 fixtures in `crates/senseid/tests/fixtures/gitlab/`, recorded with curl
from a gitlab.com Free group and a Premium trial group (spec build step 1).

**Next command:** `/sensei:build #366` once the fixtures are in.
