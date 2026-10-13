# Data-model fix — impact & conflict register

The **canonical** model is `docs/architecture/entity-access-model.md` (+ memories
[[reference_sensei_user_primary_model]], [[reference_universal_dereference_invariant]]). Everything
below either **conflicts** with it (reword/fix) or is a **code/schema impact** of aligning to it.
Resolving these is a prerequisite to planning the build (nothing may contradict canon first).

Canonical rules being enforced:
- **A. Access:** user/membership-primary for personal work (`/you`: runs, inbox, projects,
  contributions), spanning all the user's dōjōs; tenant-primary ONLY for governance + org console.
- **B. Dereference:** universal, always-on, no opt-out, ALL work (not client-only).
  `attribution_mode = named | anonymous` (credit only); `dereferenced` is NOT a mode.
- **C. Normalization:** derive `user_id` from `membership_id` (drop the column); `dojo_url` on the
  tenant (drop the membership copy); `engagements.client` → `client_tenant_id` + `client_name`.

---

## Part 1 — SCHEMA + CODE changes (the actual fixes; TDD each)

### 1A. Access: membership-derived RLS + user-wide personal reads (Rule A)
- DDL: `dojo/relay_sessions.ddl` (drop `user_id` + `user_idx`; policy → `owns_membership(membership_id)`;
  add `membership_idx`), `dojo/relay_inbox.ddl` (same), `dojo/relay_segments.ddl` (repoint join
  through the session's membership), new `owns_membership()` function. Fix RLS comments (L55–57/78,
  L47–49/62) to the membership-derived model.
- **REVISED 2026-07-28 — also drop `tenant_id` from `relay_sessions`/`relay_inbox`; key on
  `membership_id` only** (no stale/dangling copies; both `user_id` + `tenant_id` derive from the
  membership FK). Write identity `unique(tenant_id, run_id)` → `unique(membership_id, run_id)`,
  `onConflict: membership_id,run_id`. Org-console read filters `membership_id IN (memberships of
  tenant X)`. Governance/org tables KEEP `tenant_id`. See RLS design §6.
- Code: `dojo/…/relay/session/+server.ts` (stop writing `user_id`), the `relay_inbox` insert path,
  audit `relay-push-send.ts` for stored-`user_id` reads.
- Personal read goes **user-wide**: `(inbox)/+layout.ts` must aggregate the user's runs across ALL
  memberships, not `listRuns(tenantKey)` (single tenant). Detail in
  `docs/design/2026-07-27-dojo-relay-rls-membership-function.md`.
- `tenants.ddl` L20–22 comment: reframe "the service filters every query by tenant" → tenant is the
  governance/org + isolation axis, NOT the primary personal-read filter.

### 1B. Universal dereference (Rule B) — ROOT: `enum/dojo/attribution_mode.ddl`
- DDL: `attribution_mode.ddl` → `enum('named','anonymous')` (drop `dereferenced` + its client-only
  gloss). Update every consumer's comment/default to credit-only: `dojo/memberships.ddl` L46,
  `dojo/policies.ddl` L26, `sensei/dojo_memberships.ddl` L31/L54, `sensei/collective_preferences.ddl`
  L19/28/40, `sensei/dojo_inbox.ddl` L36, `dojo/artifacts.ddl` L45/L55/L56-57, `dojo/upstream_queue.ddl`
  L28, `dojo/engagement_status.ddl` L4, `dojo/engagements.ddl` L21-32.
- Code: `crates/dojo-protocol` `AttributionMode` (drop `Dereferenced`; the wire `dereferenced: bool`
  becomes an always-true invariant / removed as an option), `crates/senseid/src/dojo/{attribution,
  routing,contribute,client,memberships}.rs` (make the publish path ALWAYS run `dereference()`,
  remove `mode==dereferenced` branches), and the dojo attribution UI/data:
  `dojo/src/lib/{admin,client,dojo}-data.ts`, `{admin,client}-view.ts`, `server/artifacts-data.ts`,
  `components/DojoSignIn.svelte`, `components/kit/fixtures.ts`. The `AttributionMode` TS union →
  `'named' | 'anonymous'`.
- **Invariant to test:** a contribution with `attribution_mode = named` is STILL source-dereferenced
  (compiler-enforced completeness on the Rust enum drop; cargo test in `attribution.rs`).

### 1C. Normalization (Rule C)
- `dojo/memberships.ddl` L7: drop `dojo_url` (derive via `tenant_id → tenants.dojo_url`); repoint any
  dōjō-side reader. (`sensei.dojo_memberships.dojo_url` is a local daemon cache — keep.)
- `dojo/engagements.ddl` L4: `client text` → `client_tenant_id uuid references dojo.tenants(id)`
  (nullable) + `client_name text not null`; update readers + `dojo-lead-console.md` L40.

---

## Part 2 — DOCUMENTATION conflicts to reword (audit result)

**ROOT: `docs/spec/pipeline/dojo-lifecycle.md`** (L104 `attribution_default (named|anonymous|
dereferenced)`; the origin table L134-141 "Personal → nothing stripped / Client → dereferenced";
L143 "dereference for client work is automatic"; L212). Every screen spec cites this — fix it first.

Rule B (dereference client-only / treated as a mode) — reword to universal + credit-only:
- `docs/spec/screen/observatory-collective.md` L35/L44 · `observatory-dojo-sharing.md` L23/L34-35/L43/L49
  · `observatory-share-review.md` L32/L38-40/L49/L58 · `observatory-upgrades.md` L31/L68/L84
  · `dojo-developer-console.md` L25/L39/L49 · `project-memories.md` L50/L61 · `project-about.md` L73-74
  · `spec/pipeline/memory.md` L119-120 · `features/05-governance.md` L100.
- Minor/soft: `spec/pipeline/collective-intelligence.md` L45 · `spec/pipeline/insights.md` L146
  · `spec/agents/README.md` L96-97 · `spec/README.md` L194 · `architecture/dojo.md` L70.
- Historical logs (lower priority, mark as superseded): `spec/park/_dojo-build-plan.md` L39 ·
  `spec/park/_run-state.md` L1294-1297.

Rule C: `dojo-lead-console.md` L40 (client shown as a bare name → `client_name` + `client_tenant_id`).

Access (Rules A): **0 doc conflicts** — the access-model docs already state user-primary.

## Part 3 — MEMORY conflicts (mine; fixed in this pass)
- `project_standalone_completion_plan.md` L13 — claimed generated memories never reference source →
  "no dereference needed." FALSE premise; reworded (universal dereference stays on any share).
- `project_beta_relay_plan.md` L14 ("client-strip") + L43 (personal relay tenant-scoped) — reworded to
  universal dereference + user-primary relay.

---

## Highest-leverage order
1. `enum/dojo/attribution_mode.ddl` (schema root) + `dojo-lifecycle.md` (doc root) → collapses ~40 of
   the dereference conflicts.
2. `tenants.ddl` access comment + the relay RLS (1A).
3. Everything else follows mechanically (compiler-guided on the Rust enum drop).

Sequencing/TDD plan comes AFTER these conflicts are confirmed resolved (per the directive: conflict
resolution is part of impact identification).
