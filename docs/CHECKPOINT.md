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
| 1. 236 | dōjō access: views + RLS, user session | steps 1–3 DONE + LIVE; step 4 plan `docs/plan/2026-10-12-dojo-device-plane-as-person.md`; 4a relay `7a259d90`, projects `b6c219f8`, artifacts `b90e15ac`, `tenant_pack_rules` `2cfc77eb` — 4a DONE; 4b DONE (`3b2af4ea`…`5f715460`; rules/resolved fixed — `[]` since 2026-08-25); 4c DONE (`c6f5c068` `/v1/you/memberships`, `431b7617` link table, `c4c8b667` binding); next: 4d |
| 2. 225 · 239–241 · 245 · 252 | team view · productionise · e2e red gate | — |
| 3. 254 | next ORM, call_sites volume | measure |

**Next command:** #236 step 4d — `DojoClient::for_membership` (`crates/senseid/src/dojo/client.rs`) and `federation/mod.rs` present `live_access_token(slot)` when `membership_persona` is bound (signed out → error, never the device token); unbound → device token (the window). Then release N + live relay round trip with a session token. Owed prod check for the window: `select count(*) from dojo.memberships where device_token_hash is not null`. **Prod:** LIVE — DB (reconciled; `team` origin) and Worker `1a5ef88` (2026-10-12 19:22, built by Cloudflare from the `main` push). Cloudflare Workers Builds fixed: `dojo` Worker root `/dojo`, `bun run build`, main → `wrangler deploy`, other branches → `wrangler versions upload`; deploy = merge develop → main via PR (main is protected). Private advisories: all CLOSED (r872, 2f6q, jjcc, 3956, gxvr, hqv4 — fixed and live; m3c6 withdrawn).

**Hand-applied to live DB** (installs need a release): `schema_*` tables + enums,
`call_sites`, task kinds `import_dbd_schema` / `derive_orm_schema`. The daemon
is the DEBUG build (`make install-debug`, which ends in `cargo clean`).
Untracked files from another session (`src/`, `docs/components/`,
`docs/rune-usage.md`, `svelte5-runes-*.md`) — not touched.

**Known broken:** e2e #245. Plugin with PostToolUseFailure unpublished until
`make bump`. `commit_scans.tips` stores duplicates (noted on #253). SQL reader
ignores design.yaml's stated dialect. Old hashes in docs/*.md predate the rewrite.
