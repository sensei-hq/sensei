---
title: Dōjō full surface — blueprint (desktop + mobile)
description: The complete dōjō surface to mock parity plus the net-new model/router/key-vault surface, so the remaining build is mechanical vertical slices. Covers fixture→real wiring, the keys/model surface on a shared credential-vault crate, state coverage, mobile IA, the run-detail chat tab, and the plan→auth-gated track-work URL handoff.
type: blueprint
status: blueprint
created: 2026-07-27
depends_on:
  - docs/design/dojo-web.md
  - docs/analysis/2026-07-27-mock-vs-impl-gap-analysis.md
  - docs/blueprints/2026-07-27-shared-credential-vault.md
references:
  - dojo/src/routes/(app)/
  - dojo/src/lib/components/kit/
  - dojo/src/lib/relay-data.ts
  - ~/Developer/strategos/monorepo/apps/admin/src/routes/(app)/connections/+page.svelte
  - ~/Developer/strategos/monorepo/packages/core/src/auth/session.svelte.ts
---

# Dōjō full surface — blueprint (desktop + mobile)

## Objective

Design the whole dōjō surface once — every screen, desktop and mobile, with its states — so the
remaining build is **mechanical vertical slices** (DDL → daemon/Worker `/v1` route → mapper →
screen → test), not fresh design. Adds the **net-new model/router/key-vault surface** on a
**[shared credential-vault crate](2026-07-27-shared-credential-vault.md)** with tight key security.
No code here.

## Current state (post-F1–F5)

The engine, relay, and org admin consoles are **real end-to-end**; personal governance and org
overview/knowledge/scopes are **fixture** pending `/v1` read-routes + DDL; model/router/keys **does
not exist** anywhere.

| Bucket | Surfaces |
|---|---|
| **WIRED (real e2e)** | auth · Inbox · Run detail (outline/graph/activity, realtime) · gate reply · My dōjōs · org Triage · Approvals · Incidents · Client-audit · Members/Roles/Audit · Identity · Health |
| **PARTIAL (real + fixture)** | Org Home (real stats, fixture projects/needs) · Engagements (real list, fixture confidentiality) · Billing (real seats, fixture tiers/invoices) |
| **FIXTURE-ONLY** | personal Constitution · Rule packs · Project preview · org Constitution/ladder · org Projects · Scopes · Knowledge |
| **STUB (honest-empty)** | personal Projects · Contributions |
| **MISSING (net-new)** | model/router **Connections & Keys** surface (personal + org) |

## Target surface — three zones

### Zone A — Personal (`/you/*`) — the landing, always in reach
Inbox (✅) · Projects (wire) · Constitution (wire) · Rule packs (wire) · My dōjōs (✅) ·
Contributions (wire) · Project preview (wire) · **Connections & Keys (net-new, personal BYOK)**.

### Zone B — Org (`/org/[slug]/*`) — role-scoped
Home (partial→wire needs/projects) · Constitution/ladder + **authoring** (maintainer, net-new
write path) · Projects (wire) · Triage/Approvals/Knowledge (maintainer; knowledge=wire) ·
Engagements/Incidents/Client-audit (lead) · Members/Roles/Scopes/Identity/Billing/Health (admin;
scopes+billing-tiers=wire) · **Connections/Models/Routing (admin, net-new)**.

### Zone C — Model / router / keys (net-new, personal + org)
The dōjō analog of `@seiki/admin`'s connections/models/routing:
- **Connections** — BYOK provider keys (connect/rotate/revoke) + issued API identities. Personal
  scope on `/you/connections`; org scope on `/org/[slug]/connections` (`connection.manage`).
- **Models** — per-scope model catalog enable/disable (`/…/models`, `model.manage`).
- **Routing** — fallback-chain step activation (`/…/routing`, `chain.read`/`chain.write`).
- Backend = the sensei daemon consuming the **[shared credential-vault crate](2026-07-27-shared-credential-vault.md)**;
  read via `GET /v1/{connections,models,routing}`, write via `POST /rpc/{connections,models,routing}/*`.

## State coverage — every data-backed screen

The mock specifies **no loading/error states**; only filter-miss empties exist. Each data screen
gets three artboards + the app gets five critical paths:

- **Per screen:** loading (skeleton) · error (ret/relay-unreachable banner) · honest-empty.
- **Critical paths (app-wide):** `404`/not-found · `+error.svelte` boundary · **403 permission-denied**
  (role-gated URL) · **401 session-expired → hard re-auth** · **429 rate-limit**. None exist today.

## Mobile IA (resolves open **Q6**)

Journey-map shell = **bottom tab bar**. Proposal, personal context:

`Inbox · Projects · Connections · More`  (More = Constitution · Rule packs · Contributions ·
My dōjōs · org switch · sign-out).

Org context swaps the tabs to `Home · Triage · Members · More` gated by role; the **org switcher**
lives in More on phone (top-bar popover on desktop). Run detail, project preview, and the keys
screens are full-screen pushes. All layout via rokkit responsive `md:` (mobile-first).

## The plan → auth-gated *track-your-work* URL handoff

When local sensei **registers a plan / starts a run**, surface a **clean handoff**: a URL the user
opens to watch that run in the dōjō, gated behind their dōjō sign-in.

- **Today:** the daemon already federates a run → local dōjō (`relay_sessions`/`relay_segments`),
  watchable at `/you/runs/<run_id>`; the run-detail screen is WIRED.
- **Net-new:** at `register_plan`/`start_run`, the daemon emits the URL
  (`https://dojo.sensei-hq.com/you/runs/<run_id>` or the member's local dōjō host) + a one-line
  "track your work here" message in the plan/run output. The URL resolves only for the authenticated
  owner (membership + tenant scope already enforced by `guardTenantScope`); an unauthenticated hit
  → `/signin?redirect=`. This is the clean plane handoff: sensei plans locally, the dōjō watches.
- **Design points:** which host (cloud `dojo.sensei-hq.com` vs the member's federated local dōjō —
  from `dojo/memberships`); whether the URL is per-run or a stable per-plan tracker; the message
  surface (plan skill output / MCP `register_plan` return / a notification).

## Run-detail chat tab

Adopt `kit/ChatThread` + `toKitChatThread` (`relay-map.ts`) — both built — as a Chat tab on the run
detail (the mock has it; the impl doesn't). Needs **human-turn reply history** to be honest: today
the mapper only recasts segments as sensei turns; wire the viewer's replies (`relay_segments`
direction / a replies read) so the thread shows both sides.

## Card cleanup (verified vs the current mockup)

- **Delete** `kit/RunCard` + `kit/DecisionCard` (+ their harnesses + the `relay.spec.ts` cases) —
  superseded by `InboxRow` and `RelayGateCard`/Triage/Approvals; never rendered in the current mock.
- **Keep** `kit/ChatThread` (the chat tab above) and `kit/NeedsYouBand` (already live on `ScrOrgHome`).

## Workstreams, sequencing & dependencies

| WS | What | Depends on | Notes |
|---|---|---|---|
| **W1 · Wire fixture→real** | projects · contributions ledger · constitution/stance · rule packs · knowledge · scopes · org projects · confidentiality · billing tiers | Tier-3 DDL + `/v1` read-routes | the mechanical bulk; per-surface vertical slices |
| **W2 · Keys/model surface** | Connections/Models/Routing (personal+org) | **shared credential-vault crate** (hand-off to strategos via `sensei-hq/gateway` issues) | net-new; client ports from `@seiki/admin`; decrypt only in daemon |
| **W3 · States + chat** | loading/error/empty + 5 critical paths + run-detail chat tab | — (parallelizable) | designer artboards + build |
| **W4 · Track-work URL** | plan→auth-gated run URL handoff | — (relay already federates) | small, high-UX; **lead per Jerry** |
| **W5 · Mobile/responsive** | bottom-tab IA + `md:` pass across all zones | screens exist | folds through as each screen lands |

**Sequencing (per Jerry, 2026-07-27):** **W4 first** (+ small wins), **W1 in parallel** (mechanical
wiring; file its backend slices as `sensei-hq/gateway`/daemon issues where cross-repo), **W2 gated on
the vault crate** (blueprinted separately, handed to the strategos session), **W3/W5 fold in** as
screens land. Overall approach = **design-complete first** (this blueprint), then build.

## Reuse map — strategos → dōjō

| dōjō need | Source (strategos) | Verdict |
|---|---|---|
| Connections/keys UI | `apps/admin/.../connections/+page.svelte` + `api.ts` connect/rotate/revoke/issue | ADAPT (near-copy; swap `@torii/ui`→dōjō rokkit atoms) |
| Auth/session client | `@torii/core` `session.svelte.ts` (`SessionStore`) + admin `authHeader/gwGet/gwPost` + `onUnauthorized` | REUSE / copy pattern |
| Models/routing UI | `apps/admin/.../{models,routing}/+page.svelte` | ADAPT (load→group→toggle→403-aware) |
| Kit tokens | `@torii/ui` (Rokkit + presetRokkit, cites sensei's `rokkit.config.js`) | same family; token markup ports 1:1 |
| Key vault (backend) | `services/gateway/src/{crypto,vault,state}.rs` | → the shared crate (see the vault blueprint) |

## Client-security non-negotiables (carried from `@seiki/admin`)

1. **Never render a secret** — GET shapes expose `connected`/`connected_at`/`prefix`/`last_used_at`
   only; the pasted key is a local `$state` on `<input type=password autocomplete=off>`, sent
   write-only, cleared on return.
2. **Reveal-once** for issued keys — shown once ("won't be shown again"), then nulled; list refreshed
   from the masked endpoint; server stores an argon2id hash.
3. **Bearer discipline** — every privileged call carries `Authorization: Bearer <JWT>`; writes only
   via `/rpc/*`; the client is **not** a trust boundary.
4. **401 → hard re-auth** — sign out + full-page redirect to `/signin` (re-entrancy latched) so a
   `claims_version` bump / device revoke tears down in-memory auth.
5. **403 → capability message** — attempt-then-explain; the server is the only gate.

## Open decisions

- **Personal-vs-org key scope** — model "personal" as a personal tenant vs a null-tenant convention
  (strategos is strictly org-tenant). Affects the vault's tenant key + RLS.
- **Client capability gating** — preemptive hide/disable vs strategos's deliberate server-gates-then-
  explains. Recommend matching strategos (simpler, one gate).
- **Track-URL host** — cloud `dojo.sensei-hq.com` vs the member's federated local dōjō host; per-run
  vs per-plan tracker.
- **N1** rule-packs screen name (was "Library"); **Q6** mobile tabs (proposed above).
