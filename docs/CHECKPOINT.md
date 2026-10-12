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
| 1. 236 | dōjō access: views + RLS, user session | steps 1–3 DONE — every JWT-plane route reads/writes as the signed-in person (#236 comment 2026-10-12 lists the commits); `Caller.db` gone `dd83e96e`; next: step 4 device token → JWT |
| 2. 225 · 239–241 · 245 · 252 | team view · productionise · e2e red gate | — |
| 3. 254 | next ORM, call_sites volume | measure |

**Next command:** #236 step 4 — device token → short-lived JWT for its principal, so the device plane (artifacts, rules, the daemon's relay session/segments/inbox POSTs, still `resolveApiKeyAccess` + service role) runs under the same policies. Start from `docs/spec/dojo/access-matrix.md` "Sequence" 4 and `resolveApiKeyAccess` in `dojo/src/lib/server/dojo-auth.ts`. **Prod owed (owner), in order:** `dbd deploy --scope dojo` (31 policy files, ~12 new definer functions, `set_pack_adoption` signature change, `all_my_repositories` grant narrowed) → confirm `select has_schema_privilege('authenticated','dojo','usage')` is true (else the one-time grant in `docs/plan/decisions.md`) → `select * from dojo.backfill_shared_rules_tenant()` (report `unprovable`) → Worker deploy immediately after. Private advisories: GHSA-gxvr + GHSA-r872 fixed, wait on deploy; new GHSA-hqv4 (org-name squatting via create dōjō, low) needs a product decision. Verified: the dojo scope applies from an empty DB; 29/29 dojo DB tests pass there.

**Hand-applied to live DB** (installs need a release): `schema_*` tables + enums,
`call_sites`, task kinds `import_dbd_schema` / `derive_orm_schema`. The daemon
is the DEBUG build (`make install-debug`, which ends in `cargo clean`).
Untracked files from another session (`src/`, `docs/components/`,
`docs/rune-usage.md`, `svelte5-runes-*.md`) — not touched.

**Known broken:** e2e #245. Plugin with PostToolUseFailure unpublished until
`make bump`. `commit_scans.tips` stores duplicates (noted on #253). SQL reader
ignores design.yaml's stated dialect. Old hashes in docs/*.md predate the rewrite.
