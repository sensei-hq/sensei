# Checkpoint

**State 2026-10-14.** v0.13.0 shipped (the dōjō takes the person's session
only; production 401s a device token). History scrub done. Working the queue
on `develop` (`ade610a6`), toward 0.14.0.

| increment | status |
|---|---|
| History scrub: transcript review + 5 contributor names out of all history | DONE — force-pushed develop/main/6 tags; PR refs need GitHub Support purge |
| #266 session retrospective as an independent capability | FILED (later) |
| #241 org-home needs band = triage queue; health alerts "not measured" | DONE `da9b5419` (sparkline/needs/billing wait on sources) |
| #252 a team sees THAT a run is going (status only) | DONE `6176afd0` |
| #240 provable user-scoped metrics reach the dōjō | DONE `79f14390`, `6cd03d62` |
| #245 app e2e gate | CLOSED — 144/0/19; boot gate fix, Instruments servers+toggle restored, kind chip retired |
| #254 next ORM | MEASURED: JPA; spec 20 §8 written (`9eb44cdf`); next 8b annotation fact |
| #225 dōjō graph sync | DŌJŌ SIDE DONE: tables + file_graph_slices (`a84aa8fc`), default_branch capture (`0f35f6ac`), set_graph_branch (`64949f44`), hash read (`237ab3d1`), /v1/you/graph (`9bf0141a`), plan branches (`ade610a6`); dbd#65 filed; left: schema_*/folders tables, DAEMON side (manifest/diff/slices/batch/push per tracked checkout), app 'tracked branch' setting |

**Next command:** #225 daemon side — `crates/senseid/src/tasks/dojo_sync.rs`: after the metrics push,
for each allowed repo's `graph_branches`, each local checkout (folders.repository_id, folders.branch)
on one → manifest, GET hashes, slices of changed files, batch ≤4 MB, POST /v1/you/graph.

**Open questions (user's):**
- Discovered-MCP-servers section and per-server toggle were dropped in the
  Instruments redesign — restore, or retire their two e2e tests?
- `deploy-dojo` for 0.13.0 awaits approval; the `device_token_hash` drop needs an
  attended `dbd reconcile --scope dojo --allow-destructive`.
- Stray Cloudflare build config `1ad95662…` (trigger "develop") still fails the
  "Workers Builds: dojo" check — delete it in the dashboard.
- Sign in under Settings → Dōjō on this machine: nothing syncs until then (#184).

**Known-broken:** e2e green; 0.14.0 needs `deploy-dojo` before
the Worker can file metrics (`dojo.file_metric_rows`).
