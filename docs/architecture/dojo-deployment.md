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

## 2. dojo-api → **not** a Worker (`dojo-api.sensei-hq.com`)

`dojo-mind` (binary `sensei-dojo`) is a **Rust/axum** service — it can't run on Cloudflare Workers or Pages (those are JS/WASM). It needs a Rust-capable host. So `dojo-api.sensei-hq.com` is a **separate deploy**, not a route inside the web app.

Options:
- **Separate subdomain (recommended)** — host `sensei-dojo` on Fly.io / Railway / Render / a container, point `dojo-api.sensei-hq.com` at it. Clean separation; the web app calls it via `PUBLIC_DOJO_API_URL`.
- **Proxy under the web app** — a SvelteKit `/api/*` route in the Worker could forward to dojo-mind (avoids CORS + a subdomain), but dojo-mind **still** needs a Rust host somewhere. Not worth it initially.

Whichever host: give it the same Supabase (env `DATABASE_URL` or `SUPABASE_DB_URL` — dojo-mind reads either) and bind `SENSEI_DOJO_BIND=0.0.0.0:$PORT`. The `dojo.*` schema is already deployed (20 tables + scopes + global tenant).

> Until dojo-mind is hosted, the web app deploys fine but its data calls fail (it defaults `PUBLIC_DOJO_API_URL` to `http://127.0.0.1:7755`). The R9–R11 console screens are greenfield, so that's expected — deploy the web app now, stand up dojo-api when you wire live data.

## 3. Supabase

- Project **sensei-hq** (`lagwuqrtshjtlcuvjfnd`, us-east-1, Postgres 17), linked via `supabase link`.
- `.env` (git-ignored) holds `SUPABASE_URL`, `SUPABASE_KEY`, `DATABASE_URL`.
- Schema push: `dojo-mind` deploys the `dojo` scope on boot — `set -a; source .env; set +a; cargo run -p dojo-mind` (reads `DATABASE_URL`). Already done.
