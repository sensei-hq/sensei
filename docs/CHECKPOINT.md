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
| 1. 236 | dōjō access: views + RLS, user session (kavach#51 upstream) | steps 1–2 DONE; 3a relay reads `5147005d`; 3b relay writes `94326ec7` + push subs/prefs `49be7037`; projects `691b0622` (`callerDb`); rules write floor → maintainer `446306a9`; knowledge `fafc14c0` (+ triage embed fix `5fe7da81`); engagements `9d40fc65`; audit + ledger `5b823ba4`; incidents `e36b9971` (+`a335e8b8`); memberships + identities `f834bd00`; invites `89469e46`; health `6027698b`; billing `54d116e0`; every `/v1/t` JWT-plane route moved; `/you` projects `82b4e6a6`, contributions `10ea8226`, console chrome `640e8187`, repositories + sync plan `4a6f8f97` (`my_repositories()`), register `f7ce8b19`, elections `975b6856` (definer functions); next: metrics → rule packs → create dōjō |
| 2. 225 · 239–241 · 245 · 252 | team view · productionise · e2e red gate | — |
| 3. 254 | next ORM, call_sites volume | measure |

**Next command:** #236 step 3, remaining areas onto `callerDb` (the caller's JWT), one area per commit, each with a `database/tests/dojo/*.sql` policy test + probes: the rest of the personal plane — R4 metrics ingest + activation writes, rule-packs + adopt, create dōjō (via `resolveCaller`'s service `db`). RLS was OFF on `repository_elections`, `tenant_connections`, `tenant_share_policy` until `975b6856` (no anon/authenticated grants locally; owner to confirm none in prod); provision + invite accept stay system. Relay session/segments/inbox POST stay service-role until step 4 (device token → JWT). Verify with `scripts/relay-roundtrip.sh` (dev server with PUBLIC_SUPABASE_ANON_KEY). **Prod owed (owner):** `dbd deploy --scope dojo` again (new policy files: notification_prefs, push_subscriptions, projects, artifacts, triage_queue, decisions, policies, engagements, audit_events, incidents, memberships, identities, invites, seats, billing_accounts, downstream_inbox, tenants, metric_activations; predicate DDL incl. my_repositories(), register_repository(), set_repository_election(), principal_is_member + tenant_live_session_count; is_confidentiality_action + confidentiality_ledger view) → `select * from dojo.backfill_shared_rules_tenant()` (report `unprovable`) → Worker deploy. Advisory GHSA-gxvr: all items fixed, waits on deploy. Access findings stay in PRIVATE draft advisories until fixed + deployed.

**Hand-applied to live DB** (installs need a release): `schema_*` tables + enums,
`call_sites`, task kinds `import_dbd_schema` / `derive_orm_schema`. The daemon
is the DEBUG build (`make install-debug`, which ends in `cargo clean`).
Untracked files from another session (`src/`, `docs/components/`,
`docs/rune-usage.md`, `svelte5-runes-*.md`) — not touched.

**Known broken:** e2e #245. Plugin with PostToolUseFailure unpublished until
`make bump`. `commit_scans.tips` stores duplicates (noted on #253). SQL reader
ignores design.yaml's stated dialect. Old hashes in docs/*.md predate the rewrite.
