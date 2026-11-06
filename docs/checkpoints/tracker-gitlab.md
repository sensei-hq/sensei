# Checkpoint — GitLab tracker adapter (2026-11-06)

**Slice:** GitLab as a tracker (gitlab.com + self-managed, Free/CE up), daemon path. Slice 1 of
`docs/analysis/2026-11-06-forge-trackers.md`; GitHub is slice 2. Branch `feature/forge-trackers`.

**Stage:** design done, gate PASSED. Spec `docs/spec/daemon/tracker-gitlab.md` (r2, commit 4724c9486).
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

**Next command:** `/sensei:build`, starting at build step 1 (fixtures, human) and step 2 (shared seams; Jira suite green).
