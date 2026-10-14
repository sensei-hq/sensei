# Checkpoint

**State 2026-10-14.** v0.13.0 shipped (the dōjō takes the person's session
only; production 401s a device token). History scrub done. Working the queue
on `develop` (`96893f0d`), toward 0.14.0.

| increment | status |
|---|---|
| History scrub: transcript review + 5 contributor names out of all history | DONE — force-pushed develop/main/6 tags; PR refs need GitHub Support purge |
| #266 session retrospective as an independent capability | FILED (later) |
| #241 org-home needs band = triage queue; health alerts "not measured" | DONE `da9b5419` (sparkline/needs/billing wait on sources) |
| #252 a team sees THAT a run is going (status only) | DONE `6176afd0` |
| #240 provable user-scoped metrics reach the dōjō | DONE `79f14390`, `6cd03d62` |
| #245 app e2e gate | CLOSED — 144/0/19; boot gate fix, Instruments servers+toggle restored, kind chip retired |
| #254 next ORM | MEASURED: JPA; spec 20 §8 written (`9eb44cdf`); next 8b annotation fact |
| #268 dōjō diagram screens | FILED |
| #225 dōjō graph sync | dōjō side + daemon push + schema rows done (`abcc52cc`, `310682e7`, `ab978687`); folders deliberately not synced (measured); left: live verification on local Supabase test tenant (spec §6 step 6), app per-checkout status; screens → #268 |

**Next — todos in order:**
1. #225 live verification (local Supabase test tenant): `make install-debug`; `dbd reconcile`
   the live daemon DB for `sync_entity.repository_graph`; sync this repo; per-table counts
   match local; one committed edit moves only its file; a deleted file is pruned.
2. #225 app: per-checkout graph sync status from `sync_state` (`repository_graph`).
3. #268 dōjō diagram screens over the shared graph (branch picker, team scoping).
4. #254 JPA 8b — `Annotation` fact in the Java walk (red test first), then 8c/8d.
5. #238 mods — blocked: brew cask at Claude Code 2.1.287, mods need ≥ 2.1.289.

**Open questions (user's):**
- `deploy-dojo` for 0.13.0 awaits approval; the `device_token_hash` drop needs an
  attended `dbd reconcile --scope dojo --allow-destructive`.
- Stray Cloudflare build config `1ad95662…` (trigger "develop") still fails the
  "Workers Builds: dojo" check — delete it in the dashboard.
- Sign in under Settings → Dōjō on this machine: nothing syncs until then (#184).

**Known-broken / deploy order:** e2e green. 0.14.0 needs `deploy-dojo` FIRST
(`file_metric_rows`, the `graph_*` tables and functions, `/v1/you/graph`); a debug
daemon at the same version needs `dbd reconcile` for `sync_entity.repository_graph`
or its graph step errors each cycle (metrics unaffected). dbd#65 open (PK re-key
with a dependent FK; changed function return type).
