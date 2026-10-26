# Checkpoint — 2026-10-25

**Slice:** issue burndown (docs/roadmap/08-issue-burndown.md). Wave 4 is closed; wave 5 is in progress.
Released v0.20.0 (dōjō deployed, Worker `fcf5d87`) and v0.20.1 (daemon only). Develop CI green.

**Closed this run:** #106, #335, #201, #204, #183, #142, #298, #172, #195 (#120 updated).

**Landed, each closing on the next release's binary:**
- #178 (2e5c6c1f): a file over 2 MiB, or with a line over 10,000 bytes, is skipped as `oversized`.
  Local DB reconciled; 137 of 61,349 corpus files refused.
  Check: `select count(*) from sensei.files where skip_reason='oversized'` > 0.
- #188 (d5bfdc7d): `GET /api/tasks/abandoned?days=N` and a Watchdog section in `sensei doctor`.
  Live: 204 abandonments in 14 days. Check: the released `sensei doctor` prints the section.

- #336 (e5bf2e9d): a Claude transcript is read once per ingest, not five times; 25.2 s → 5.1 s on
  the 1.28 GB transcript. Check: a week of `/api/tasks/abandoned` with no `ingest_capture`.

- #125 (17e81b4b): ingest only sessions under the watch roots; a `sensei.folders` trigger re-opens the
  skips when a repository is registered. Check: 0 traceable orphan turn sets (18 with no transcript stay).
- Watch roots: the live DB watches only sensei on purpose, until the indexer covers JS/TS/Java/.NET
  (user, 2026-10-25). Then ~/Developer + ~/Work; test/e2e use ~/Developer/sensei-hq.

**In progress — #161 (typed rows):** 98e2c9cf (`adopt_node_by_identity` → `FqnNodeRow`), 558b90fd (the three
MCP writers → `McpServerRow`/`McpManifestRow`/`AssistantToolRow`, with round-trip tests). The guard
`db/pg_store/tests/typed_rows_tests.rs` lists 47 still at 5+ args; take one off per conversion.

**Next:** the seven left at 7 args (`insert_edge_with_props`, `merge_function`, `callee_row`,
`insert_check_run`, `insert_consolidated_ruleset`, `upsert_folder`, `upsert_workflow_state`). Each becomes
a typed row with exhaustive destructuring and a round-trip test, and comes off `STILL_POSITIONAL`.

**Open (the user's):**
- dbd 0.27.7: the v0.27.6 lockfile pins rustls-webpki/webpki-roots/ctutils for
  `cargo install --locked` (fixed on dbd develop, 784db3d2). Needs a go.
- v0.20.1's `deploy-dojo` is still waiting; it would change nothing. Approve or reject.
- #331: count production's duplicate personal dōjōs (counts only):
  `psql "$PROD" -Atc "select count(*) from dojo.tenants where origin='personal' and key ~ '-[0-9]+$'"`.
- A release (patch) lets #178, #188, #336 and #125 close; it's the user's call when.

**Known:** DeepSource is red on kavach main; qlty is out of minutes. After a DDL change, run
`make test-template` (#142). `update_phase` returned 404 from the released daemon.
