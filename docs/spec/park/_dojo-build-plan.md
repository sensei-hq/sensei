# Dōjō track — build plan (refreshed 2026-07-13)

> Roadmap for the collective-intelligence / federation SaaS layer. Drives the build.
> DECISION locked: **Fork 1** (dojo.* lives in the Dōjō Rust service's PG; Supabase = auth ONLY).
> This refresh corrects a badly stale prior version: the "Docker-blocked" premise was FALSE
> (Docker running + Supabase CLI 2.109.1 installed), and far more is already built and tested
> than the prior doc claimed. Verified against code 2026-07-13 (file:line anchors below).

## Headline
The **entire Docker-free spine is built and tested** — DDL, both wire-protocol crates, the
multi-tenant dual-auth Dōjō service (hive-mind), the daemon-side contribute/anonymise/inbox loop,
the maintainer triage/promotion engine, and 3 of 4 desktop screens. What remains is the **SaaS
console web app + its auth plane (in-repo Supabase + kavach)**, the **admin/lead console
BACKEND endpoints** (only the maintainer surface exists on the service), the **share-review
desktop screen**, and an **upstream contribute cadence scheduler**. This is now mostly
*new-frontend + a few backend endpoint additions*, not foundational construction.

---

## STATE MAP — built vs missing (verified file:line)

### ✅ BUILT & TESTED (Docker-free, embedded PG)

**Schema (C1) — done.**
- `dojo` scope: full table set under `database/ddl/table/dojo/` (tenants, memberships, artifacts,
  triage_queue, decisions, events, downstream_inbox, upstream_queue, engagements, incidents,
  policies, roles, identities, audit_events, notifications) + 17 enums under `enum/dojo/` +
  `procedure/dojo/seed_global_dojo.ddl` (idempotent `org/global-dojo` seed).
- `dojo` scope declared in `database/design.yaml:35-43` (self-contained, vector-free, `includes:[dojo]`);
  daemon `default` scope EXCLUDES `dojo` (design.yaml:45) — Fork 1.
- Daemon-local mirrors: `database/ddl/table/sensei/dojo_memberships.ddl` (connection mirror +
  Keychain `credential_ref`, `last_seq` pull cursor), `dojo_outbox.ddl` (durable C6 send-ledger,
  `unique(membership_id, signature)` idempotent replay), `dojo_inbox.ddl` (C7 downstream inbox,
  same dedup key). `sensei.projects.dojo_id` binds a project → membership.

**Wire protocol — done.**
- `crates/dojo-protocol/src/lib.rs` (666 LOC, 20 passing tests): 6 `ArtifactKind`
  (principle/pattern/prompt/guard/skill/agent), typed payloads, `ArtifactScope`, `Attribution`
  (named/anonymous/dereferenced), `PublishedArtifact`/`PulledArtifact`/pull cursor, and
  `artifact_signature()` reusing `hive_protocol::content_hash` (DRY). Analogue of `hive-protocol`
  (rules wire types, 104 LOC).

**Dōjō service = hive-mind (C3 + C8) — done, runnable, 16 test files.**
- Binary `sensei-hive`, `crates/hive-mind/src/main.rs`: `serve` (default) | `keygen` | `provision`
  CLI. Embedded Postgres (`postgresql_embedded`), no Docker. Default bind `127.0.0.1:7755`
  (`config.rs:39`), deploys hive+dojo scopes + `seed_global_dojo` on boot (`db.rs:207,214,257`).
- Multi-tenancy (`provision.rs`): tenant key `<origin>/<org>[/<dojo>]`, origin `github|org`,
  scope `private|global`; `resolve_tenant`/`create_membership`/`generate_key` (store.rs:389-460).
- Dual auth (`auth.rs`): API-key sha256 constant-time (`require`, l.73) + Supabase JWT HS256
  (`verify_supabase_jwt`, l.144) with expiry+aud checks; `authenticate_dojo` (l.246) tries API-key
  then JWT→membership-role. Synthetic-token testable via `DEFAULT_SUPABASE_JWT_SECRET` — **no
  running Supabase needed to build/test** (`tests/jwt_test.rs`, `tests/dojo_artifacts_test.rs`).
- Tenant artifact routes (`api.rs`): `POST/GET /v1/t/{tenant}/artifacts` (publish/pull, seq cursor),
  `GET /v1/t/{tenant}/triage`, `POST …/triage/promote`, `POST …/triage/{sig}/decide`.
- Promotion engine (`collective/promote.rs`, 987 LOC): cluster-by-signature, score
  (`breadth 0.16/contributor + efficacy from ftr_delta`), auto-approve bar 0.80, k-anonymity ≥3 for
  global tenants, human-triage queue, `decide_triage` with safe-default gates (approve requires
  `distribution_scope`, decline requires reason), idempotent. Wired to the triage endpoints.

**Daemon side (C4–C7, C9 backend) — done, routes MOUNTED.**
- `crates/senseid/src/api/routes.rs:260-270` mounts: `/api/preferences/collective` (GET/PUT),
  `/api/dojo/memberships` (GET/POST), `/api/share-review/next-batch` (GET) + `/{batch}/publish`
  (POST), `/api/upgrades` (GET) + `/{id}/apply|mute|pin` (POST). Handlers in
  `api/handlers/{preferences,dojo,share_review,upgrades}.rs`.
- C4 memberships/routing/client: `dojo/memberships.rs` (register→Keychain token + insert mirror +
  optional project bind), `dojo/routing.rs` `client_precedence_route` (pure, 58 test cases) **called
  live** at `dojo/contribute.rs:412`, `dojo/client.rs` (Bearer to `/v1/t/{tenant}/artifacts`).
- C5 confidentiality: `dojo/attribution.rs` deterministic `dereference()` + fail-closed
  `residual_risk()` (20+ adversarial tests); `collective/anonymize.rs` stricter global path (strip →
  optional `reasoning`-chain LLM polish → re-verify), rotating anon id.
- C6 contribute: `dojo/contribute.rs` (1024 LOC) memory-batch → artifacts → publish via `PgOutbox`
  ledger; gate-enforced (`ItemPlan::Held` never ships). Wired to `POST /api/share-review/{batch}/publish`
  (**synchronous, manual** — see gap R1).
- C7 downstream: `collective/inbox.rs` (916 LOC) pull→mirror→apply/mute/pin; **auto-pulled every 300s**
  inside `federation/mod.rs:207-219` (the same loop spawned at `api/server.rs:298`).
- C9 backend: `collective/preferences.rs` (destination none|global|dojo|both, cadence
  manual|daily|weekly, per-category toggles, attribution default).
- Config: `crates/sensei-config/src/lib.rs:15,24` `dojo_registry_url()` / `SENSEI_DOJO_URL`
  (default `http://localhost:8787`).

**Desktop screens — 3 of 4 real & API-wired.**
- `(observatory)/dojo/sharing/` (ObsCollectiveSettings, ~180 LOC) → `/api/preferences/collective`.
- `(observatory)/dojo/connections/` (~193 LOC) → `/api/dojo/memberships`.
- `(observatory)/upgrades/` (~249 LOC) → `/api/upgrades` (+ local recos lane). `buckets.ts` is UI
  bucketing, not mock data.

**kavach (external, ~/Developer/kavach, pre-release).** `@kavach/adapter-supabase` (GitHub OAuth,
magic-link OTP, password, session sync, RLS-via-Bearer), `@kavach/sentry` declarative RBAC route
guard, `@kavach/cli`+vite scaffold. Local supabase template at `~/Developer/kavach/supabase/`
(`config.toml` ports api 54321 / db 54322 / studio 54323 / **inbucket 54324**, magic-link enabled
with `enable_confirmations=false`; `seed.sql` test users with `app_metadata.role`).

### ❌ MISSING / GAPS (must build)

| # | Gap | Evidence |
|---|---|---|
| G1 | **SaaS console web app** — sign-in + org-picker + maintainer/admin/lead consoles | no `console/`/`saas/` dir anywhere; `website/` is marketing-only; no `@kavach`/`PUBLIC_SUPABASE_URL`/`PUBLIC_DOJO_API_URL` in-repo |
| G2 | **In-repo `supabase/`** (config.toml, seed.sql, migrations) | none in repo; no `supabase`/`inbucket` make targets (`make hive` is the only Dōjō target) |
| G3 | **Admin console BACKEND endpoints** on hive-mind — members list/role-set, identities/SSO config, policies CRUD, health rollups, audit-events list | tables exist (`dojo.{roles,identities,policies,audit_events}`) but `api.rs` mounts none; provisioning is CLI-only (`sensei-hive provision`) |
| G4 | **Lead console BACKEND endpoints** — engagements CRUD, dereferenced-artifact audit view, incidents CRUD, compliance export | tables exist (`dojo.{engagements,incidents,audit_events}`) but no HTTP surface |
| G5 | **Share-review desktop screen (C11)** | daemon API exists (`api/handlers/share_review.rs`) but no `(observatory)/share-review/` route |
| G6 | **Upstream contribute cadence scheduler** — auto-prepare/publish on daily/weekly cadence per prefs | only manual `POST …/publish`; only `run_pull_loop` (downstream) is spawned. `observatory-collective.md` done-gate wants cadence to auto-fire |
| G7 | **Auto-bind at project detect** + project-About binding chip | `bind_project` is user-explicit via connect handler (`dojo.rs:100`); lifecycle spec wants git-remote heuristic auto-bind (InappBind mockup) |
| G8 | **Bootstrap seed catalogue** for downstream lane on install + community-vs-personal metrics on Impact/Today | collective-intelligence.md done-gate; no seed catalogue found |
| G9 | **Auto-discovery** (`.well-known/dojo` probe) + first-run join prompt (InappJoin) | not built (parked; real DNS deferred) |
| G10 | **Port reconciliation** — hive binds `7755`, `sensei-config` default registry is `8787` | `config.rs:39` vs `sensei-config/src/lib.rs:15` |
| G11 | hive `/v1/subscriptions` webhook push | `api.rs:186` returns **501** (intentional placeholder) |

---

## AUTH ARCHITECTURE (adopted) = DUAL-PLANE — unchanged, now partly built
- **Humans → Supabase via kavach**, ONLY in the NEW SaaS console app. Providers: magic-link (primary
  local, Supabase Inbucket:54324) + GitHub OAuth (PARKED — needs a real OAuth app). `@kavach/sentry`
  gates `/console/{maintainer,admin,lead}`.
- **Dōjō service (hive-mind) accepts BOTH** — already built: Supabase JWT (verify→sub=user→
  membership+role) for console traffic; API-key/device-token for daemon/federation traffic; tenant
  from `<origin>/<org>/<dojo>` path.
- **Desktop app + senseid daemon: NO kavach, NO Supabase** — already true. Console mints a
  per-membership device token; user pastes into the desktop connect flow → reuses Keychain
  `credential_ref`. Daemon is credential-BEARING, not credential-ISSUING.
- **localhost registry** — `SENSEI_DOJO_URL` (default 8787; reconcile with hive's 7755 — G10).
  Console env `PUBLIC_DOJO_API_URL` + `PUBLIC_SUPABASE_URL=http://localhost:54321`.

---

## REMAINING BUILD CHUNKS (dependency-ordered; risk-tagged)

Legend: 🟢 SAFE-AUTONOMOUS (schema/config/scaffold/backend/UI, no live auth — buildable + verifiable
unattended) · 🔴 AUTH/CREDENTIAL-SENSITIVE (Supabase auth flows, kavach, secrets/tokens — needs
Jerry's eye; NEVER touch `.env`/real credentials).

### Track P — close the already-shipped desktop loop (no auth, no Docker)
- **R1 🟢 Upstream contribute cadence scheduler (G6).** Background task honoring
  `collective_preferences.{destination,cadence,mode}`: on daily/weekly tick, prepare + (mode=auto)
  publish or (mode=review) stage approved batches; reuse `contribute::contribute_batch` +
  `preview_batch`; persist a watermark like the analyzer scheduler. Spawn beside `run_pull_loop`.
  *Verify:* unit test cadence/mode gating; integration that a due batch auto-contributes and `off`
  never ships; `curl /api/share-review/next-batch` reflects it.
- **R2 🟢 Share-review desktop screen (G5, C11).** New `(observatory)/share-review/` over
  `/api/share-review/*`; mockup `dojo-inapp.jsx` `InappShare` (l.220) + batch-history "watch it
  travel" `InappTravel` (l.300); spec `observatory-share-review.md`. *Verify:* svelte-check 0 +
  Playmright e2e (injected batch → Publish → row moves to sent).
- **R3 🟢 Auto-bind at detect + project-About chip (G7).** Call `client_precedence_route`/heuristic
  at project detect to set `projects.dojo_id`; About binding chip (`InappBind`, confirm-inferred).
  *Verify:* unit test heuristic; e2e binding chip.
- **R4 🟢 Bootstrap seed catalogue + peer metrics (G8).** Ship a seed catalogue that lands in the
  downstream lane on install; community-vs-personal comparison on Impact/Today. *Verify:* fresh
  install shows seeded Upgrades items; metric renders with ≥30d data.

### Track S — SaaS foundation (un-parks the console; Jerry pre-authorized Supabase-localhost + kavach)
- **R5 🟢 In-repo `supabase/` scaffold (G2).** Port `~/Developer/kavach/supabase/` in: `config.toml`
  (rename `project_id`), `seed.sql` (test users w/ `app_metadata.role`), magic-link + Inbucket on;
  add `make supabase-up/down` (localhost only). NO real secrets; NO GitHub OAuth secret.
  *Verify:* `supabase start` boots the stack; Studio:54323 + Inbucket:54324 reachable (Jerry may run
  the container step; the files themselves are config-only 🟢).
- **R6 🔴 Console app scaffold + auth plane (G1, C2).** New SvelteKit app (proposed `console/`):
  `kavach init` + `@kavach/adapter-supabase`; `@kavach/sentry` guards `/console/*`; env
  `PUBLIC_SUPABASE_URL` + `PUBLIC_DOJO_API_URL`. Sign-in (magic-link) + org-picker (`dojo-saas.jsx`
  `DojoSignIn` l.43 / `DojoOrgs` l.192). *Scaffold + static render = 🟢; live magic-link login vs
  Inbucket = 🔴 (Jerry verifies).* Consumes PUBLISHED `@kavach/*` (external-dep rule — no inline
  kavach edits; MVP needs none).
- **R7 🟢 Admin console BACKEND endpoints on hive-mind (G3).** Add `/v1/t/{tenant}/…`:
  `members` (list + role-set), `identities` (SSO/GitHub/device-code config), `policies` (CRUD),
  `health` (connections/queue-depth/publish-rate/error-rate rollups from `dojo.events`), `audit`
  (audit_events list). Dual-auth, admin-role floor. *Verify:* synthetic-JWT integration tests (no
  Supabase), role-floor 403s, done-gate curls from `dojo-admin-console.md`.
- **R8 🟢 Lead console BACKEND endpoints on hive-mind (G4).** `engagements` (CRUD + project
  bind), `artifacts` audit view (`dereferenced=true` filter), `incidents` (CRUD + severity/SLA),
  compliance export (CSV/PDF-ready, strip-covered columns only). *Verify:* the
  `non_dereferenced == 0` curl gate from `dojo-lead-console.md`; export leaks no source refs.

### Track C — SaaS console screens (frontend over Track S; each behind kavach/sentry)
- **R9 🔴 Maintainer console** (queue/candidate/evaluate/decide/distribute/measure) over the EXISTING
  triage endpoints. Mockup `dojo-console.jsx` `DojoOverview`(l.191)/`DojoTriage`(l.373)/
  `DojoCandidate`(l.451). UI is 🟢; the auth gate makes the runnable path 🔴.
- **R10 🔴 Admin console** (members/roles/identities/policies/monitor/audit) over R7. `DojoMembers`(l.649).
- **R11 🔴 Lead console** (engagements/audit/incidents/dereference-verify/compliance export)
  over R8. `DojoClients`(l.709).

### Track H — hardening / parked
- **R12 🟢 Auto-discovery** `.well-known/dojo` probe + `InappJoin` first-run prompt (real DNS deferred).
- **R13 🔴 GitHub OAuth** provider (needs a real OAuth app; magic-link stays primary local).
- **R14 🟢 Port reconciliation (G10)** — pick 7755 vs 8787, align `sensei-config` + docs + console env.
- **R15 🟢 Subscriptions/webhooks (G11)** — replace hive `/v1/subscriptions` 501 with real push
  (lower priority; 300s pull already works).

**Execution order:** {R1, R2} (parallel, close the loop) → R3 → R7+R8 (console backends, parallel) →
R5 → R6 → {R9, R10, R11} → R4 / R12 / R14 / R15 (polish) → R13 (deferred).

---

## FIRST 3 BUILDABLE CHUNKS (🟢 — hand to a build subagent; no Docker, no auth)

### R1 — Upstream contribute cadence scheduler
- **Read first:** `docs/spec/pipeline/collective-intelligence.md` (done/wrong gates),
  `docs/spec/screen/observatory-collective.md` (cadence chip), `dojo/contribute.rs`
  (`contribute_batch` l.557, `preview_batch` l.615, `load_batch` l.393), `collective/preferences.rs`
  (destination/cadence/mode enums), `api/server.rs:206-214` + `tasks/analyzer_scheduler.rs`
  (watermark/tick pattern to mirror), `db/pg_store.rs:5579` (batch creation) — confirm who
  creates/approves `memory_share_batches`.
- **Create:** `crates/senseid/src/tasks/contribute_scheduler.rs` (a `spawn(pg, interval)` mirroring
  `analyzer_scheduler`); wire `crate::tasks::contribute_scheduler::spawn(...)` in `api/server.rs`
  beside line 298. Gate on `preferences.destination != none` and `cadence`; `off`/`manual` no-op.
- **Verify:** `cargo test -p senseid` (new unit tests: cadence due-calc, mode gating, `off`→empty);
  an integration test that a due `daily` batch flows through `contribute_batch` and lands rows in
  `dojo_outbox` as `sent`/`held`; `make test-fast` green; zero-errors-policy.

### R2 — Share-review desktop screen (C11)
- **Read first:** `docs/spec/screen/observatory-share-review.md`, mockup
  `docs/mockups/Sensei/lib/dojo-inapp.jsx` `InappShare`(l.220)+`InappTravel`(l.300),
  `MOCKUP-INDEX.md` (`/share-review` → `InappShare`), existing sibling
  `(observatory)/upgrades/+page.svelte` + its `dojo-upgrades-state.svelte.ts` as the state/API
  pattern, `api/handlers/share_review.rs`.
- **Create:** `app/src/routes/(observatory)/share-review/+page.svelte` + `+page.ts` +
  `share-review-state.svelte.ts`; add `getShareReviewBatch()`/`publishBatch()` to `app/src/lib/api.ts`
  + types in `types.ts`; register the `share-review` section in the observatory shell/nav. Invoke the
  `svelte-file-editor` agent / Svelte MCP (mandatory for `.svelte`). Rokkit named tokens only.
- **Verify:** `svelte-check` 0; Playwright e2e (injected next-batch → renders items + org-policy bar →
  Publish → batch history "travels"); zero-errors-policy.

### R7 — Admin + lead console BACKEND endpoints on hive-mind
*(Do R7 & R8 together — same service, same test harness; both 🟢, synthetic-JWT testable.)*
- **Read first:** `docs/spec/screen/dojo-admin-console.md` + `dojo-lead-console.md` (done
  gates + curls), `crates/hive-mind/src/api.rs` (route + dual-auth pattern, `authenticate_dojo`),
  `store.rs` (dojo CRUD helpers to extend), `tests/dojo_promote_test.rs` + `dojo_artifacts_test.rs`
  (synthetic-JWT harness to copy), DDL `dojo.{roles,identities,policies,engagements,incidents,audit_events,events}`.
- **Create:** new handlers + store methods for members list/role-set, identities, policies CRUD,
  health rollups, audit list (admin); engagements/incidents CRUD, dereferenced-artifact audit view,
  compliance export (lead). Admin-role floor via `DojoAccess`.
- **Verify:** `cargo test -p hive-mind` new tests (role-floor 403s; `approve` needs
  `distribution_scope`; lead audit `non_dereferenced == 0`); embedded PG only, no Docker;
  keep all 16 existing hive tests green; zero-errors-policy.

*(R5 in-repo `supabase/` scaffold is the 🟢 on-ramp to Track S — config files only, safe to add any
time; the `supabase start` container run is the only step that may want Jerry.)*

---

## OPEN QUESTIONS FOR JERRY
1. **Console app location & name.** New top-level `console/` SvelteKit app in this monorepo, a
   subtree, or a separate repo? (Marketing `website/` is separate from this.) Affects R6 scaffold.
2. **Port reconciliation (G10).** Standardize the Dōjō service on `7755` (hive default) or `8787`
   (`sensei-config` default)? Pick one; console `PUBLIC_DOJO_API_URL` + docs follow.
3. **Contribute default on cadence (R1).** For `mode=auto`, should the scheduler *auto-publish*
   eligible batches, or only *stage* them and still require a human Publish? (Confidentiality posture.)
4. **Deploy target.** Is `dojo.sensei-hq.org` (SaaS) a near-term deploy, or is localhost-only the
   scope for now? Affects whether R6/R9-R11 need production Supabase + real GitHub OAuth (R13).
5. **Console↔service auth for reads.** Console calls hive with the user's Supabase JWT directly
   (built), or via a thin BFF? Direct is built and simplest; confirm.
6. **Seed catalogue source (R4).** Who curates the bootstrap community insights, and in what format
   (a checked-in JSONL landed via `seed_global_dojo` + publish, or a fixture the daemon imports)?

---

## RISKS / PARKS (still valid)
- **HARD confidentiality gates** (already built + heavily tested; keep them so): client-work
  dereference fail-closed (`attribution.rs`), global k-anonymity ≥3 (`promote.rs`), scope-match
  before local write (`inbox.rs`), durable outbox replay (`dojo_outbox`). Any new endpoint that
  emits artifacts MUST route through these — never add a bypass.
- **kavach edits: MVP needs NONE.** Console consumes published `@kavach/*` (external-dep rule). If
  device-code/SSO later → separate kavach PR + republish, not an inline monorepo edit.
- **GitHub OAuth local** needs a real OAuth app → magic-link (Inbucket) is primary local; GitHub PARKED (R13).
- **Can't fully verify locally** (assume-localhost): DNS multi-tenant routing, cross-tenant isolation
  under load, `.well-known/dojo` discovery, per-tenant crypto isolation.
- **KEEP senseid unauthenticated-by-design** (localhost/Tauri-trusted). Do NOT add Supabase to senseid.
- **Never touch `.env`/real credentials.** Local Supabase uses dev secrets only; production secrets
  are Jerry's, out of scope for autonomous chunks.
