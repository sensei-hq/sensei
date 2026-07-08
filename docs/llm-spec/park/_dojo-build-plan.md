# Dōjō track — build plan (scoped 2026-07-08)

> Roadmap for the collective-intelligence / federation SaaS layer. Drives the build.
> Full scoping in agent transcript a12cdab3c506a3d1e. DECISION: **Fork 1** (below).

## Headline
The **governance-rule federation SUBSTRATE is fully built & shipped**; the **Dōjō SaaS layer on top is
almost entirely ABSENT**. This is new construction on a solid foundation, not retrofit.

## What EXISTS (real, verified)
- `crates/hive-mind` (`sensei-hive` bin): Axum + embedded PG, `hive` dbd scope. Endpoints `/v1`:
  health, POST/GET /rules (publish/pull-by-seq cursor), DELETE /rules/{id} (tombstone), POST /members,
  /members/{id}/keys, subscriptions→501 stub. Auth = API-key (sha256, constant-time). roles member|
  publisher|admin. `keygen` bootstrap. Tested.
- `crates/hive-protocol`: shared serde wire types + content_hash normalizer.
- Daemon federation: `crates/senseid/src/federation/mod.rs` — push_promoted (on accept_proposal),
  pull_source, run_pull_loop (spawned 300s). API /api/knowledge/sources[/{id}][/sync|/status]. Creds in
  OS Keychain via gateway_keys credential_ref.
- Federation DDL: sensei.{knowledge_sources,federated_memories}; hive.{shared_rules,members,api_keys,
  audit_log}; `hive` scope in database/design.yaml.
- generalise endpoint (anonymisation primitive — shipped v0.2.27).
- promote_memory/accept_proposal + memory_share_batches + status enum.
- Config UI: (observatory)/knowledge-sources/+page.svelte (real).
- **kavach** (~/Developer/kavach, published pre-release): @kavach/adapter-supabase REAL e2e (GitHub OAuth,
  magic-link OTP, password, session sync, RLS-via-Bearer). @kavach/sentry = declarative RBAC route guard.
  @kavach/cli+vite scaffold kavach.config.js → $kavach/* virtual modules. Local supabase scaffold at
  ~/Developer/kavach/supabase/ (config.toml, seed.sql test users, app_metadata.role).

## What's ABSENT (must build)
- `dojo.*` schema (memberships/tenants/triage_queue/artifacts[6 types]/decisions/events/downstream_inbox/
  upstream_queue/engagements/audit_events/incidents/roles/identities/policies). NO dojo scope.
- User/org identity: NO user_id/org_id/scope_user_id/scope_org_id anywhere in DDL. projects has NO dojo_id.
- Multi-tenancy in hive (today "one instance == one org"); no <origin>/<org>/<dojo> routing, no RLS.
- crates/senseid/src/dojo/* and .../collective/* (none exist).
- All Dōjō daemon APIs (/api/dojo/*, /api/upgrades, /api/preferences/collective, /api/share-review/*).
- Console web app (SaaS frontend); no in-repo Supabase wiring.
- Artifact model: federation carries RULES ONLY; 6 Dōjō artifact types (principle/pattern/prompt/guard/
  skill/agent) not modeled.
STUBS: (observatory)/upgrades (local buckets.js, no API); hive /v1/subscriptions (501).

## AUTH ARCHITECTURE (recommended + adopted) = DUAL-PLANE
- **Humans → Supabase via kavach**, ONLY in the NEW SaaS web console app. kavach init +
  @kavach/adapter-supabase; providers = magic-link (primary local, uses Supabase Inbucket) + GitHub OAuth
  (PARKED for local — needs real OAuth app). @kavach/sentry gates /console/{maintainer,client-lead,admin}.
- **Dōjō service (evolved hive-mind) accepts BOTH**: Supabase JWT (verify→sub=user→membership+role from
  dojo.memberships) for console traffic; existing API-key/device-token for daemon/federation traffic.
  Tenant from <origin>/<org>/<dojo> path; isolation in-query (Fork 1).
- **Desktop app + senseid daemon: NO kavach, NO Supabase.** MVP: console mints a per-membership device
  token after Supabase login → user pastes into desktop app → REUSE existing knowledge_sources + Keychain
  credential_ref flow VERBATIM. Zero new auth code in the daemon. (Biggest simplifier — keeps the shipped
  federation boundary: daemon is credential-BEARING, not credential-ISSUING.)
- **localhost registry** = one config value `dojo_registry_url` in crates/sensei-config (env
  SENSEI_DOJO_URL, default http://localhost:8787), mirrors SENSEI_DDL_DIR pattern. Console env
  PUBLIC_DOJO_API_URL=http://localhost:8787, PUBLIC_SUPABASE_URL=http://localhost:54321. Daemon's
  create_source guard already permits localhost http. Auto-discovery short-circuited to localhost (real
  DNS discovery PARKED). kavach changes needed for MVP = NONE (device-code + SSO deferred).

## ⭐ DESIGN FORK — RESOLVED = **FORK 1** (default-and-proceed, 2026-07-08)
Where does dojo.* data physically live?
- **FORK 1 (CHOSEN):** dojo.* in the Dōjō Rust service's Postgres (evolved hive, new `dojo` dbd scope in
  the same DDL tree). Supabase = auth/identity ONLY. Preserves the Rust federation + hive-protocol
  investment; daemon↔Dōjō stays clean hive-protocol; tenant isolation = in-query filter/service-role.
  RATIONALE: matches user's literal "supabase for AUTH" (not all-data-in-supabase); preserves shipped
  substrate; reversible (Chunk 1 is DDL). If Jerry prefers Fork 2 (dojo.* in Supabase RLS, console via
  kavach /data/dojo/*, hive demoted to rule-sync bridge) → mainly shrinks Chunk 3 + moves schema to
  Supabase migrations.

## BUILD CHUNKS (dependency-ordered; ≈1 delegated agent each)
Phase 0 — foundations:
  C1  dojo.* DDL + dbd `dojo` scope + alter sensei.projects add dojo_id + dojo_notifications + seed
      global-dojo scope. [DDL] (Fork resolved → unblocked)
  C2  local supabase + kavach console scaffold (new SvelteKit app; magic-link+GitHub; sentry /console/*;
      localhost config; prove login vs Inbucket). [infra+frontend] (parallel w/ C1)
  C3  Dōjō service multi-tenancy + dual auth (evolve hive-mind: Supabase-JWT middleware alongside API-key;
      tenant from path; deploy dojo scope; per-tenant isolation; membership/role). [backend] (needs C1,C2)
Phase 1 — personal-side (daemon):
  C4  senseid/src/dojo/{memberships,routing}.rs + /api/dojo/memberships + project binding + client-
      precedence routing + creds via gateway_keys + localhost discovery. (needs C1,C3)
  C5  dojo/attribution.rs + collective/anonymize.rs — universal client-work DEREFERENCE + global-dojo
      anonymisation (reuse generalise/reasoning). HARD confidentiality gate → heavy tests. (needs C4)
  C6  upstream contribute/share-review: memory_share_batches→Dōjō push; extend protocol rules-only→6
      artifact types (recommend NEW dojo-protocol crate); /api/share-review/next-batch, upstream queue.
      (needs C4,C5)
  C7  downstream inbox/distribution — collective/inbox.rs: pull approved→land per type (principle/pattern
      →rules/memories origin=dojo; skill/agent/prompt→plugins; guard→lint); mute/pin; /api/upgrades; seed
      catalogue. Extends run_pull_loop. (needs C4)
Phase 2 — collective loop:
  C8  collective/promote.rs (Dōjō-service side): cluster-by-signature, score, auto-approve, human-triage
      queue, community catalogue, k-anonymity. Serves maintainer console. (needs C3,C6)
Phase 3 — desktop observatory screens (thin over daemon API):
  C9  Preferences→Sharing (observatory-collective) + /api/preferences/collective. (needs C5,C6)
  C10 dojo-connections + dojo-sharing over /api/dojo/*. (needs C4)
  C11 share-review screen + replace upgrades stub (Apply/Mute/Pin). (needs C6,C7)
Phase 4 — SaaS console screens (kavach/sentry-protected web app):
  C12 maintainer console (queue/evaluate/decide/distribute/measure). (needs C3,C8)
  C13 admin console (stand-up/identity/provisioning/policies/monitor). (needs C3)
  C14 client-lead console (engagements/audit/incidents/dereference-verify/compliance export). (needs C3,C5)

## RISKS / PARKS
- GitHub OAuth local needs a real OAuth app → use MAGIC-LINK (Inbucket) as primary local; GitHub PARKED.
- Artifact protocol: recommend NEW `dojo-protocol` crate (keep shipped rule protocol stable).
- kavach edits: MVP needs NONE. If device-code/SSO later → separate kavach PR + republish (external-dep
  rule), NOT inline monorepo edit. Console consumes PUBLISHED @kavach/* (or workspace link).
- Local Supabase state (supabase/ project, containers) via `supabase start` — infra outside app.
- Can't verify locally (assume-localhost, PARKED): DNS multi-tenant routing, cross-tenant isolation under
  load, .well-known/dojo discovery, per-tenant crypto isolation.
- HARD test gates (spec wrong-gates): client-work dereference (strip project_id/session_ids/identifiers
  BEFORE anything leaves machine — C5); collective k-anonymity (C8); distribution scope-match + mute/pin
  before local write (C7); durable upstream queue replay on reconnect (C6).
- KEEP senseid unauthenticated-by-design (localhost/Tauri-trusted). Do NOT add Supabase to senseid.

## EXECUTION ORDER: C1+C2 (parallel) → C3 → C4 → {C5,C7} → C6 → C8 → screens (C9-C11) → consoles (C12-C14).

## ── NO-DOCKER PIVOT (2026-07-08) ──
Docker is UNAVAILABLE in this env → `supabase start` (local Supabase stack) CANNOT run. Therefore:
⛔ C2 (local Supabase + kavach console + live login verification) = PARKED (needs Docker → Jerry / a
   Docker-capable env). The console app CODE could be scaffolded but not run/verified → low value now.
✅ Everything else is Docker-INDEPENDENT and proceeds: hive-mind uses EMBEDDED postgres (no Docker);
   Supabase-JWT verify is unit-testable with SYNTHETIC tokens (no running Supabase needed to build/test
   the verify+tenant-resolution logic); the daemon-side collective-intelligence pipelines are pure Rust.
REORDERED (Docker-free value path):
  C1 ✅ done (schema `37f30527`).
  → dojo-protocol crate (NEW, additive, 6 artifact-type wire types + content_hash — mirrors
    hive-protocol; prerequisite for C3/C6). [BUILDING NEXT]
  → C3 Dōjō service (evolve hive-mind: multi-tenancy + dual auth [API-key path REAL + tested; Supabase-JWT
    path synthetic-token tested]; deploy dojo scope to embedded PG). Additive — keep shipped hive tests green.
  → C4 daemon memberships/routing → {C5 dereference/anonymize, C7 downstream inbox} → C6 upstream → C8
    collective promote. All Rust, no Docker. This delivers the collective-intelligence LOOP.
PARKED-for-Docker: C2 console + live Supabase auth; C9-C14 SaaS console screens (need the console app +
running service). Desktop observatory Dōjō screens (C9-C11) partly buildable (thin over daemon API) once
C4-C7 land — they don't need Docker (daemon API + Tauri), only the SaaS *web console* (C12-C14) does.
