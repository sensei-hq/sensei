# Dōjō deployment — Cloudflare Workers + Supabase

How the Dōjō pieces deploy and connect. Two deployables + one database.

```
 dojo.sensei-hq.com          dojo-api.sensei-hq.com          Supabase (cloud)
 ┌─────────────────┐  /v1/…  ┌──────────────────────┐  SQL  ┌──────────────┐
 │ dojo web app    │───────► │ dojo-mind (sensei-    │─────► │ project      │
 │ SvelteKit Worker│  data   │ dojo, Rust/axum)     │       │ sensei-hq    │
 │ kavach SSR auth │         │ federation API       │       │ dojo.* schema│
 └────────┬────────┘         └──────────────────────┘       └──────▲───────┘
          │ auth (magic-link) via kavach + @kavach/adapter-supabase │
          └─────────────────────────────────────────────────────────┘
```

- **Auth** — the web app talks to Supabase Auth directly through kavach (`@kavach/adapter-supabase`), using `PUBLIC_SUPABASE_URL` + `PUBLIC_SUPABASE_ANON_KEY`.
- **Data** — the web app calls **dojo-mind's** `/v1/t/{origin}/{org}/…` endpoints (`src/lib/dojo-api.ts`), *not* Supabase directly. dojo-mind owns the federation logic and connects to Supabase over the Postgres `DATABASE_URL`.

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

**Environment variables** (Worker → Settings → Variables, Production):

| Var | Value |
|---|---|
| `PUBLIC_SUPABASE_URL` | `https://lagwuqrtshjtlcuvjfnd.supabase.co` |
| `PUBLIC_SUPABASE_ANON_KEY` | Supabase dashboard → Project Settings → API → **anon public** key |
| `PUBLIC_DOJO_API_URL` | `https://dojo-api.sensei-hq.com` *(the dojo-mind service, below)* |

Then add the custom domain `dojo.sensei-hq.com` (Worker → Settings → Domains & Routes → Add custom domain).

> **Supabase Auth redirect URLs** — in the Supabase dashboard (Authentication → URL Configuration) add `https://dojo.sensei-hq.com` to the allowed redirect/site URLs so magic-link sign-in returns to the deployed app.

## 2. dojo API → **in the Worker** (no separate host)

**Decision:** the `/v1` API lives *inside* the dojo Worker as SvelteKit server routes (`dojo/src/routes/v1/t/[origin]/[org]/…/+server.ts`) talking to Supabase directly — so the SaaS is **one deployable**, no Rust host. `dojo-mind`'s `/v1` surface is ~80% CRUD over `dojo.*` + a promotion sweep + JWT auth; none of it is Rust-specific.

- **Auth plane** — `lib/server/dojo-auth.ts` (`resolveTenantAccess`) ports dojo-mind's `resolve_tenant_access`: a Supabase JWT (`Authorization: Bearer` for the **desktop/API** plane, or the kavach session `access_token` for the **console**) → `sub` matched to `dojo.memberships.user_id` → role → access floor (`member<contributor<lead<maintainer<admin`). This is the **shared-auth** plane: the desktop app authenticates against the same Supabase project (PKCE/device flow) and calls authorized publish/subscribe endpoints with its JWT.
- **Data** — `lib/server/dojo-supabase.ts` is a service-role client scoped to the `dojo` schema; routes enforce authz in code (RLS is a hardening follow-up).
- **Pub/sub** (future) — Supabase Realtime for live push (new artifacts, triage, notifications) + the `seq`-cursor pull for offline catch-up; wrap an auth'd channel in kavach (upstream issue).

**Supabase prerequisites** (one-time, dashboard): Settings → API → **Exposed schemas**: add `dojo` (+ `sensei`); and set the Worker env `SUPABASE_SERVICE_ROLE_KEY` (private). Then `PUBLIC_DOJO_API_URL` is left **unset** — the console calls itself same-origin.

`dojo-mind` (the Rust binary) is kept for **local dev + the eventual cross-dojo federation endpoint** (a change-cursor pull between separate dojos), not as a hosted service for the console. `dojo-api.sensei-hq.com` is **not needed**.

> Status: the `engagements` resource is ported as the reference pattern (`ccd08bc2`). Remaining console resources — incidents, members, identities, policies, triage — follow the same three-line shape (`resolveTenantAccess` floor → `dojoDb().from(table)` → JSON).

## 3. Supabase

- Project **sensei-hq** (`lagwuqrtshjtlcuvjfnd`, us-east-1, Postgres 17), linked via `supabase link`.
- `.env` (git-ignored) holds `SUPABASE_URL`, `SUPABASE_KEY`, `DATABASE_URL`.
- Schema push: `dojo-mind` deploys the `dojo` scope on boot — `set -a; source .env; set +a; cargo run -p dojo-mind` (reads `DATABASE_URL`). Already done.
