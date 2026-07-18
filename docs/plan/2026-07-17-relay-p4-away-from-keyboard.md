---
name: Relay P4 — away-from-keyboard (push · realtime · offline)
description: Phased implementation plan for relay-engine P4 — Web Push, Supabase realtime swap, offline/reconnect, and the "what's blocked on me" home.
date: 2026-07-17
status: in-progress
---

# Relay P4 — away-from-keyboard

**Goal:** a backgrounded phone gets a **push** when the autonomous run needs the
human (gate / stall / crash) → the human answers → the engine proceeds; and while
watching, the surface updates **live** (realtime, not action-refresh) and degrades
**gracefully offline** (drafts queued, sent on reconnect). Closes the
away-from-keyboard gap left after P3 (the engine runs; the human just isn't tethered).

Design source: [`relay-engine.md`](relay-engine.md) §Liveness, §Offline, P4 row
(line ~471). Supersedes nothing; extends the P2 phone surface + P3 engine.

## Where we start (surveyed 2026-07-17)

- `@supabase/supabase-js` is already a dojo dep → client-direct realtime is feasible.
- `dojo.push_subscriptions` + `dojo.notification_prefs` DDL exist (from P0), unused.
- Transport today: the console/phone refresh via `invalidateAll()` **on user action
  only** — no live subscription, no push. "Realtime swap" = add a live subscription
  that refreshes on change; "push" = notify while backgrounded.
- No service worker / PWA manifest yet (`dojo/static/` is just a favicon).
- Worker relay routes live under `/v1/t/[origin]/[org]/relay/*`
  (session·segments·inbox·reply·review·gates·nudge).

## Infra / secrets prerequisites — AUTONOMOUS vs JERRY-GATED

P4 is the first phase that touches secrets + cloud config. Explicit split so the
autonomous build never handles a prod secret or silently changes cloud infra
(cf. [[feedback_external_dep_issue]], [[feedback_no_build_against_live_dev]]):

| Prerequisite | Autonomous (dev/local) | Jerry-gated (prod) |
|---|---|---|
| **VAPID keypair** (Web Push) | I generate a **dev** keypair; public key in dev config, private key in a **local-only** `.env` (gitignored, never committed) | **prod** VAPID keys → Jerry sets them as Worker secrets (`wrangler secret`); never in git |
| **Supabase Realtime publication** on `dojo.relay_*` | enable on **local** supabase (config/migration) | **prod** publication enablement → Jerry / infra |
| **RLS policies** on `dojo.relay_*` | author + test policies against **local** supabase | **prod** apply + validation → Jerry (RLS was a deferred hardening item; P4.1 pulls it forward because client-direct realtime needs it) |

**Rule:** I build + test everything against local supabase/dev keys, and raise a
short checklist of the prod cloud steps for Jerry rather than touching prod.

## Design fork (decided, flag for review)

**Realtime transport: client-direct Supabase Realtime + RLS** (chosen) vs
Worker-mediated SSE. The written design (relay-engine §2, line 74/442) says "phone
subscribes Supabase Realtime", and the phone is already a Supabase-JWT client
(`resolveTenantAccess`). So: phone subscribes directly, RLS enforces per-user row
access. Alternative (Worker holds the realtime conn, phone connects via SSE) avoids
pulling RLS forward but adds Worker infra and diverges from the doc. **Chosen:
client-direct + RLS** — matches the doc and the phone's existing JWT-client role.
If Jerry prefers Worker-SSE, P4.1/P4.2 change; flagged here.

## Chunks (ordered; per-chunk cadence = TDD → build/test → reviewer → commit `develop`)

### P4.1 — relay RLS + realtime publication (local)  ·  DDL
- **What:** RLS policies on `dojo.relay_sessions` / `relay_segments` / `relay_inbox`
  so a signed-in user can `SELECT` only rows for runs in a Dōjō they're a member of
  (join through `dojo.memberships`); add those tables to the `supabase_realtime`
  publication (local). The Worker keeps its `service_role` write path (bypasses RLS);
  RLS only governs the new client-direct **read/subscribe** path.
- **Acceptance:** with a local anon/authed JWT, a member `SELECT`s their run's inbox
  rows and gets them; a non-member gets zero rows; the Worker's service-role writes
  still succeed; the tables appear in the realtime publication. Tested via a psql/JWT
  harness against local supabase.
- **Deps:** none. **Autonomous** (local). Prod apply → Jerry checklist.
- **Out of scope:** prod RLS rollout; RLS on non-relay `dojo.*` tables (separate item).

### P4.2 — realtime swap (phone/console)  ·  `.svelte`/`.ts` (svelte MCP + rokkit)
- **What:** a `relay-realtime.ts` helper that opens a supabase-js Realtime channel on
  the signed-in user's relay rows; the relay run-list + run-detail pages subscribe on
  mount and refresh (`invalidateAll()` or a store patch) on `INSERT`/`UPDATE`;
  unsubscribe on unmount. Replaces action-only refresh with live updates.
- **Acceptance:** with two clients, an inbox insert (a raised gate) appears on the
  watching client within ~1–2s with no user action; the "last progress N min ago"
  clock ticks from live `run_event`/session updates; unsubscribe on navigate-away (no
  leaked channels). Unit-test the pure subscribe/patch logic; build for SSR.
- **Deps:** P4.1. **Autonomous** (local). **svelte MCP + svelte-file-editor +
  rokkit-components + semantic-styles-rokkit mandatory** for the `.svelte` edits.
- **Out of scope:** push (P4.3/4.4); offline (P4.5).

### P4.3 — Web Push: service worker + subscribe + store  ·  PWA + Worker route
- **What:** a PWA manifest + a service worker (`push` → `showNotification`,
  `notificationclick` → focus/open the run/gate); a subscribe flow (fetch the VAPID
  **public** key from a Worker config route → `PushManager.subscribe` → POST the
  subscription to a new `/v1/relay/push/subscribe` Worker route → `push_subscriptions`);
  a minimal notification-prefs opt-in (events) writing `notification_prefs`.
- **Acceptance:** a user enables notifications → a `push_subscriptions` row is written
  with `{endpoint,p256dh,auth}`; the SW registers; a locally-triggered test push shows
  a notification and tapping it opens the right run. Dev VAPID public key wired; the
  send is P4.4.
- **Deps:** dev VAPID keypair (I generate). **Autonomous** for client + store + route.
- **Out of scope:** the actual send-on-gate (P4.4); native APNs/FCM (P5+).

### P4.4 — Web Push send from the Worker  ·  Worker
- **What:** on a blocking-gate `relay_inbox` insert (and stall/crash events), the
  Worker sends a Web Push to the user's `enabled` subscriptions using a
  Worker-compatible VAPID/WebCrypto sender, **respecting `notification_prefs`**
  (events opt-in, quiet_hours, muted_tenants). Zero-knowledge payload: "needs you /
  stalled / crashed on <run>" — never code/diffs.
- **Acceptance:** a raised gate → the backgrounded phone receives a push → tapping it
  opens the gate card → Approve/Deny → the daemon's `await_reply` sees the answer →
  the engine proceeds (the P4 headline round-trip). Quiet-hours/muted respected.
- **Deps:** P4.3; the VAPID **private** key (dev local; **prod = Jerry secret**).
- **Out of scope:** batching/digests; retry/backoff on push failure (track).

### P4.5 — offline / reconnect / session-ended  ·  PWA
- **What:** local draft store for per-segment review + queued replies/guidance; a
  Send-batch on reconnect; session-ended + reconnect UX; the surface reads durable
  `relay_inbox`/`relay_segments` when offline (cached).
- **Acceptance:** airplane-mode → read the outline, mark approve/request-changes,
  queue a nudge → reconnect → Send delivers the batch → the engine applies queued
  guidance at the next safe boundary; a closed/re-opened app restores state.
- **Deps:** P4.2 (realtime for reconnect signal), P2 review UI. **Autonomous** (local).

### P4.6 — "what's blocked on me" home  ·  `.svelte` (svelte MCP + rokkit)
- **What:** a home view aggregating open gates / needs-you across the user's runs
  (the away-from-keyboard landing: "here's what's waiting on you"), ordered by
  urgency, each linking to its gate card.
- **Acceptance:** with gates open on 2+ runs, the home lists exactly the open
  needs-you items across runs, newest/most-urgent first; answering one removes it
  live (via P4.2 realtime). Empty state when nothing's blocked.
- **Deps:** P4.2. **Autonomous** (local). svelte MCP + rokkit mandatory.

## Sequencing

`P4.1 (RLS+publication)` → `P4.2 (realtime swap)` → `P4.3 (push client)` →
`P4.4 (push send)` → `P4.5 (offline)` → `P4.6 (blocked-on-me home)`.
P4.3 is independent of P4.1/4.2 and can interleave. Each chunk commits to `develop`
under approach A (no main-merge/bump). End-of-P4: `sensei-security-reviewer`
(RLS policy correctness, push payload zero-knowledge, VAPID secret handling) +
`semgrep`; dojo `bun run check`/`build`/`test` per chunk.

## Prod checklist (hand to Jerry at end of P4 — do NOT do autonomously)
- Generate prod VAPID keypair; set the private key as a Worker secret; public key in prod config.
- Enable Supabase Realtime publication on `dojo.relay_*` in prod.
- Apply + validate the relay RLS policies in prod.
- (If native later) APNs/FCM credentials.
