---
title: dōjō device plane runs as the person (#236 step 4)
date: 2026-10-12
status: approved (decisions 2026-10-12)
issue: 236
---

# The daemon's tenant calls run as the signed-in person

Step 3 moved every route a **person's session** calls onto that session, so the
policies in `database/policies/dojo/` decide. What still runs as `service_role`
is the **device plane**: the daemon's per-tenant calls, authenticated by a
membership's device token (`resolveApiKeyAccess` → `dojoDb()`).

## Decisions (2026-10-12)

| Question | Decision |
|---|---|
| What the daemon presents | **The persona's existing Supabase session** — the access token `live_access_token(persona)` already mints from the Keychain refresh token for `/v1/you/*`. No new token type; no JWT signing secret in the Worker. |
| Rollout | **One release window.** The dōjō accepts either credential for one daemon release; the next release removes device-token auth and drops `memberships.device_token_hash`. |
| Membership → persona | **Recorded at join.** `sensei.dojo_memberships.persona_id → sensei.personas.id`, written when a membership is created; existing rows backfilled once from each persona's `/v1/you/dojos`. |

Observed while deciding: nothing in today's dōjō ISSUES device tokens (the
issuer left with dojo-mind; only `scripts/relay-roundtrip.sh` seeds one). The
window may protect nobody — the owner can check
`select count(*) from dojo.memberships where device_token_hash is not null`.

## Choosing the path is explicit, not a fallback

A tenant route reads the bearer's SHAPE: a JWT (three base64url segments) goes
through `resolveTenantAccess` + `callerDb`; anything else through
`resolveApiKeyAccess` + `dojoDb()` (legacy, removed in N+1). A JWT that fails
verification is a **401** — never retried as a device token, so one credential
can never quietly stand in for the other.

## What each device-plane route writes, and who decides it

| Route | As the person (policy) | Stays the system's (matrix service-role table) |
|---|---|---|
| `POST relay/session` | `relay_sessions` insert/update on own membership; `projects` insert/update own (owner = caller, tenant one they belong to) | seat open/refresh (billing bookkeeping); crash push |
| `POST relay/segments` | `relay_segments` insert/update on own session — the agent's columns only (not the reviewer's verdict) | — |
| `GET relay/inbox` | own rows — existing read policy | — |
| `POST relay/inbox` | `relay_inbox` insert `agent_to_human` on own session | — |
| `POST artifacts` | `artifacts` insert: own submission (`contributed_by` = caller, `submitted`, member) | clustering after a publish (`promoteCluster`) |
| `GET artifacts` | existing read policy (published · own · maintainer) | — |
| `POST rules` / `DELETE rules/{id}` | `shared_rules` — existing maintainer write policies | the namespace the rule files under (`sensei.namespaces` upsert); the audit row |
| `GET rules` | existing member read | — |
| `GET rules/resolved` | — | resolved pack rules for the caller's namespaces (cross-namespace `sensei.*` read, like `my_adopted_pack_slugs`) — a pinned definer function |

## Slices (each red-first, mutation-probed, deployed twice locally)

1. **4a — policies** for the daemon's writes (table above), each with a
   `database/tests/dojo/*_access.sql`.
2. **4b — dōjō** `resolveTenantCaller`: picks the path by credential shape;
   the device-plane routes move to it, JWT → `callerDb`.
3. **4c — daemon `persona_id`**: DDL + write at join + one-time backfill.
4. **4d — daemon credential**: `DojoClient` and federation present the persona's
   access token when the membership has a persona; the device token otherwise
   (the window).
5. **Release N**, verified live with `scripts/relay-roundtrip.sh` run with a
   session token instead of a device token.
6. **4e (release N+1)** — remove `resolveApiKeyAccess`, the device-token path in
   the daemon, and `dojo.memberships.device_token_hash`.

## Done gate

The relay round trip, rules publish/pull and artifact publish/pull pass with the
daemon presenting a session token, and a session token for a person who is not a
member of the tenant gets 403 on every one of them.
