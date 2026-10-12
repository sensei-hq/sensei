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
| 1. 236 | dōjō access: views + RLS, user session (kavach#51 upstream) | steps 1–2 DONE (matrix, predicates, rules tenancy, 2c fixes); next step 3 route moves; DEPLOY owed |
| 2. 225 · 239–241 · 245 · 252 | team view · productionise · e2e red gate | — |
| 3. 254 | next ORM, call_sites volume | measure |

**Next command:** #236 step 3 — move routes to the user-session client area by area (start with relay: RLS own-rows already exists), incl. projects (tenant from the project↔membership binding, advisory item 5). **A dōjō DEPLOY is owed** for every #236 commit since `6dfb1ef0`: reconcile `dojo.shared_rules` → functions (incl. `artifacts_bump_seq`, `shared_rules_bump_seq`) → `dbd policies` → `select * from dojo.backfill_shared_rules_tenant()` (report `unprovable`) → Worker. Access findings live in PRIVATE draft advisories (Security tab) — never describe them publicly until fixed + deployed.

**Hand-applied to live DB** (installs need a release): `schema_*` tables + enums,
`call_sites`, task kinds `import_dbd_schema` / `derive_orm_schema`. The daemon
is the DEBUG build (`make install-debug`, which ends in `cargo clean`).
Untracked files from another session (`src/`, `docs/components/`,
`docs/rune-usage.md`, `svelte5-runes-*.md`) — not touched.

**Known broken:** e2e #245. Plugin with PostToolUseFailure unpublished until
`make bump`. `commit_scans.tips` stores duplicates (noted on #253). SQL reader
ignores design.yaml's stated dialect. Old hashes in docs/*.md predate the rewrite.
