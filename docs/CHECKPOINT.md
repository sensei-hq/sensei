# Checkpoint

**State 2026-10-12.** #251 (schema store + dbd import + EF Core derivation) and
#221 (Schema screen) CLOSED with live evidence; #253 (stale git tips after the
history rewrite) CLOSED live. Scan roots: `~/Developer/sensei-hq/sensei` plus ONE
private C# repo under `~/Work` (added for #251 7d — client names stay out of
commits/issues/docs). Open PRs: none. Branches: main + develop.

| # | item | status | to close |
|---|---|---|---|
| 251 | schema store, dbd import, EF Core | CLOSED — 9/9 EF tables, 2/2 keys live | follow-ons #254 |
| 221 | Schema / ER screen | CLOSED `89272517` | visual check vs mockup pending (no on-screen window for screenshots) |
| 253 | scan_git_history poison pill | CLOSED `801104c1` | — |
| 238 | Claude Code capture | missing PostToolUseFailure now WARNS (`6e0df121`); Critical loop stopped live | publish plugin (`make bump`); mods Steps 2–5 need the mods `.d.ts` |

**Queue, in order:**

| # | item | first step |
|---|---|---|
| 1. 236 | dōjō access: views + RLS, user session (kavach#51 upstream) | steps 1–2 DONE; 3a relay reads `5147005d`; 3b relay writes `94326ec7` + push subs/prefs `49be7037`; projects `691b0622` (`callerDb`); next: remaining areas |
| 2. 225 · 239–241 · 245 · 252 | team view · productionise · e2e red gate | — |
| 3. 254 | next ORM, call_sites volume | measure |

**Next command:** #236 step 3, remaining areas onto `callerDb` (the caller's JWT), one area per commit, each with a `database/tests/dojo/*.sql` policy test + probes: knowledge (artifacts/triage/rules/policies) → metrics → client work (engagements/incidents/audit) → tenancy (members/invites/identities/billing/health). Relay session/segments/inbox POST stay service-role until step 4 (device token → JWT). Verify with `scripts/relay-roundtrip.sh` (dev server with PUBLIC_SUPABASE_ANON_KEY). **Prod owed (owner):** `dbd deploy --scope dojo` again (new policy files: notification_prefs, push_subscriptions, projects; predicate DDL) → `select * from dojo.backfill_shared_rules_tenant()` (report `unprovable`) → Worker deploy. Advisory GHSA-gxvr: all items fixed, waits on deploy. Access findings stay in PRIVATE draft advisories until fixed + deployed.

**Hand-applied to live DB** (installs need a release): `schema_*` tables + enums,
`call_sites`, task kinds `import_dbd_schema` / `derive_orm_schema`. The daemon
is the DEBUG build (`make install-debug`, which ends in `cargo clean`).
Untracked files from another session (`src/`, `docs/components/`,
`docs/rune-usage.md`, `svelte5-runes-*.md`) — not touched.

**Known broken:** e2e #245. Plugin with PostToolUseFailure unpublished until
`make bump`. `commit_scans.tips` stores duplicates (noted on #253). SQL reader
ignores design.yaml's stated dialect. Old hashes in docs/*.md predate the rewrite.
