# Dōjō deployment — Cloudflare Workers + Supabase

How the Dōjō pieces deploy and connect. **One deployable + one database:** the
`/v1` API lives *inside* the dojo Worker (SvelteKit server routes) — there is no
separate Rust host. The `dojo-mind` Rust crate (`sensei-dojo` binary) has been
**removed**; the Worker `/v1` is the only dōjō backend, and senseid's federation
targets that Worker (`crates/senseid/src/federation`, `crates/senseid/src/dojo`).

```
 dojo.sensei-hq.com                         Supabase (cloud)
 ┌─────────────────┐                        ┌──────────────┐
 │ dojo web app    │  /v1/… server routes   │ project      │
 │ SvelteKit Worker│───────────────────────►│ sensei-hq    │
 │ kavach SSR auth │  service-role client   │ dojo.* schema│
 └────────┬────────┘                        └──────▲───────┘
          │ auth (magic-link) via kavach + @kavach/adapter-supabase │
          └─────────────────────────────────────────────────────────┘
```

- **Auth** — the web app talks to Supabase Auth directly through kavach (`@kavach/adapter-supabase`), using `PUBLIC_SUPABASE_URL` + `PUBLIC_SUPABASE_ANON_KEY`.
- **Data** — the web app calls the Worker's own same-origin `/v1/t/{origin}/{org}/…` routes (`dojo/src/routes/v1/…/+server.ts`), which own the federation logic and talk to Supabase via a service-role client. (Formerly served by the `dojo-mind` Rust service, now removed.)

## 1. dojo web app → Cloudflare **Worker** (`dojo.sensei-hq.com`)

Not a Pages project — the account's one Pages slot is the marketing website; everything else is a Worker (same as `rokkit`/`kavach`). The repo is now configured for this (`04048ea7`, mirroring `kavach/sites/learn`):

- `dojo/svelte.config.js` — picks `@sveltejs/adapter-cloudflare` when `WORKERS_CI` is set (Cloudflare Workers Builds sets it), else `adapter-auto`.
- `dojo/wrangler.jsonc` — committed so `wrangler deploy` uses it directly instead of interactive auto-config (that auto-config ran `wrangler types --check` and caused the *"Types file not found at worker-configuration.d.ts"* failure).
- `@sveltejs/adapter-cloudflare` added to devDependencies.

**Cloudflare → Workers → Create → connect the `sensei-hq/sensei` repo:**

| Setting | Value |
|---|---|
| Root directory | `dojo` |
| Build command | `bun run build` *(= `vite build`; do **not** use `wrangler types --check …`)* |
| Deploy command | `npx wrangler deploy` *(uses the committed `wrangler.jsonc`)* |
| Worker name | `dojo` *(must match `wrangler.jsonc` `name`)* |

**Environment variables** — set under **Worker → Settings → Variables and Secrets** (i.e. **runtime**), *not* the Build config's variables. kavach reads `PUBLIC_SUPABASE_URL`/`PUBLIC_SUPABASE_ANON_KEY` via `$env/dynamic/public` (`@kavach/vite`'s `auth-supabase` template), which resolves from `platform.env` at **runtime** — build-only variables are `undefined` when the Worker runs → `createClient` throws *"supabaseUrl is required"*. Redeploy after changing them.

Supabase's **new API keys** (publishable / secret) are drop-in replacements for the legacy anon / service_role keys and substitute directly into `createClient()` (both are opaque, not JWTs — see supabase discussion #29260):

| Var | Kind | Value (Supabase → Settings → API Keys) |
|---|---|---|
| `PUBLIC_SUPABASE_URL` | Variable | `https://lagwuqrtshjtlcuvjfnd.supabase.co` |
| `PUBLIC_SUPABASE_ANON_KEY` | Variable | **publishable** key `sb_publishable_…` (client-safe, RLS-gated) |
| `SUPABASE_SERVICE_ROLE_KEY` *(or `SUPABASE_SECRET_KEY`)* | **Secret** | **secret** key `sb_secret_…` (server-only, bypasses RLS — the `/v1` routes) |
| `PUBLIC_DOJO_API_URL` | — | leave **unset** — the `/v1` API is same-origin (in this Worker) |

> Publishable vs secret are **two different keys**, not interchangeable — publishable for the browser (kavach), secret for the server routes. The var *names* keep the legacy `ANON`/`SERVICE_ROLE` wording; the *values* are the new keys.

Then add the custom domain `dojo.sensei-hq.com` (Worker → Settings → Domains & Routes → Add custom domain).

> **Supabase Auth redirect URLs** — in the Supabase dashboard (Authentication → URL Configuration) add `https://dojo.sensei-hq.com` to the allowed redirect/site URLs so magic-link sign-in returns to the deployed app.

## 2. dojo API → **in the Worker** (no separate host)

**Decision:** the `/v1` API lives *inside* the dojo Worker as SvelteKit server routes (`dojo/src/routes/v1/t/[origin]/[org]/…/+server.ts`) talking to Supabase directly — so the SaaS is **one deployable**, no Rust host. The old `dojo-mind` `/v1` surface was ~80% CRUD over `dojo.*` + a promotion sweep + JWT auth; none of it was Rust-specific, so the whole surface was ported into the Worker and the Rust crate was **removed**.

- **Auth plane** — `lib/server/dojo-auth.ts` (`resolveTenantAccess`) ports dojo-mind's `resolve_tenant_access`: a Supabase JWT (`Authorization: Bearer` for the **desktop/API** plane, or the kavach session `access_token` for the **console**) → `sub` matched to `dojo.memberships.user_id` → role → access floor (`member<contributor<lead<maintainer<admin`). This is the **shared-auth** plane: the desktop app authenticates against the same Supabase project (PKCE/device flow) and calls authorized publish/subscribe endpoints with its JWT.
- **Data** — `lib/server/dojo-supabase.ts` is a service-role client scoped to the `dojo` schema; routes enforce authz in code (RLS is a hardening follow-up).
- **Pub/sub** (future) — Supabase Realtime for live push (new artifacts, triage, notifications) + the `seq`-cursor pull for offline catch-up; wrap an auth'd channel in kavach (upstream issue).

**Supabase prerequisites** (one-time, dashboard): Settings → API → **Exposed schemas**: add `dojo` (+ `sensei`); and set the Worker env `SUPABASE_SERVICE_ROLE_KEY` (private). Then `PUBLIC_DOJO_API_URL` is left **unset** — the console calls itself same-origin.

The `dojo-mind` Rust binary has been **removed** (retirement complete): the Worker `/v1` is the only dōjō backend, for the console *and* for senseid's federation (rules + artifacts ride the Worker's tenant path over the `dojo_protocol` wire — see `crates/senseid/src/federation` and `crates/senseid/src/dojo`). `dojo-api.sensei-hq.com` is **not needed**.

> Status: the `engagements` resource is ported as the reference pattern (`ccd08bc2`). Remaining console resources — incidents, members, identities, policies, triage — follow the same three-line shape (`resolveTenantAccess` floor → `dojoDb().from(table)` → JSON).

## 3. Supabase

- Project **sensei-hq** (`lagwuqrtshjtlcuvjfnd`, us-east-1, Postgres 17), linked via `supabase link`.
- `.env` (git-ignored) holds `SUPABASE_URL`, `SUPABASE_KEY`, `DATABASE_URL`.
- Schema push: the `dojo` scope was originally deployed by the (now removed) `dojo-mind` service on boot; the `dojo.*` schema now lives in Supabase and is managed alongside the rest of the DDL. Already done.

## 4. PWA + push (Relay) — *planning-only, not yet built*

Relay makes the dojo web app a surface you carry. The plan (see
[dojo.md → Relay](dojo.md#relay--through-the-dōjō) for the full model):

- **Installable PWA** — add a `dojo/static/manifest.webmanifest` + icons + a
  service worker so the responsive site installs to the home screen (Android /
  desktop auto-prompt; iOS via Share). No app-store distribution needed for the
  *experience*.
- **Two notification paths** (Realtime only works while the app is open):
  - **Web Push** (Service Worker + Push API + **VAPID** keys) → Android / desktop,
    even closed. iOS only for an *installed* PWA (16.4+), less reliable.
  - **Thin [Capacitor](https://capacitorjs.com) wrapper** loading the same PWA +
    native **APNs/FCM** — the reliable iOS push path. A config app, not a second
    codebase. This is the map's "native app coexists for push + offline".
- **Secrets** — VAPID keypair (web), APNs auth key + FCM server key (native) as
  Worker/Supabase secrets, **never in git** (this repo is public).
- **DB** — the Relay tables (`push_subscriptions`, `relay_sessions` + presence,
  `relay_inbox`, `notification_prefs`) land in the `dojo` schema; push dispatch is
  a server routine (Worker route or Supabase Edge Function) that reads a
  subscription and sends. See the [Relay data model](dojo.md#relay--through-the-dōjō).

> Status: **not started.** The `dojo/` SvelteKit app has no manifest / service
> worker / push wiring yet — deferred until the Dōjō+Relay mockups settle.
