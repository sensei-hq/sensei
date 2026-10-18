# Checkpoint — 2026-10-18

**Slice:** issue burndown (docs/roadmap/08-issue-burndown.md). Wave 0 is released; wave 1 is next.

**Done:**
- **Audit:** all 109 open issues audited; 20 closed with evidence; 47 partials carry status comments.
- **v0.14.0 released.**
  - Tag cbf61c4a. PR #273 merged as ed533c7. main CI is green.
  - Shipped assets checked: the tap's arm64 sha matches the asset; the shipped CLI reports 0.14.0.
  - deploy-dojo ran on production: 3 tables altered, 0 dropped. The stale v0.13.0 deploy run is cancelled.
  - dojo `/version` = 0.14.0.
  - Plugin 0.14.0 is installed and its mods fire from a plain install.
- **Issues:** #238 closed; #252 auto-closed; #240 reopened (live production push not yet observed).

**Remaining:**
- Wave 0: #240 needs a production-signed-in daemon to push a user row. #184 needs `sync_status` written or retired.
- Wave 1: Settings → Projects (#271, #269, #144). Data first: 13,346 keyless, folderless repository rows,
  5,618 of them linked into projects, then the API, then the UI.
- Wave 2: the user's decisions (the table in 08-issue-burndown.md).

**Next command:** `/sensei:session refocus`, then wave 1, red-first: a test that a folder prune takes its
repository row and project link with it.

**Open questions:**
- An orphaned Cloudflare build trigger (deleted Worker, tag 1ad95662…) fails every develop push. Delete it in the dashboard?
- 8 CodeQL `rust/cleartext-logging` alerts, pre-existing in test files and relocated by the tests split, need triage.

**Known-broken / pending:**
- Dependabot reports 0 alerts although the lockfiles are vulnerable (#172).
- The committed `database/design.dbml` is stale (pre-existing).
