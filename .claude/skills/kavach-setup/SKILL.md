---
name: kavach-setup
description: Use when configuring Kavach in a SvelteKit app — kavach.config.js, the @kavach/vite plugin (must precede sveltekit()), the generated $kavach/* virtual modules ($kavach/auth, $kavach/providers, $kavach/config, $kavach/routes), kavach.handle in hooks.server, the browser instance via createKavach in +layout onMount + setContext, changing the auth/data/rpc/logout/home/session paths through routes, and the kavach init / doctor CLI.
---

# Kavach Setup — Configuration & Wiring

Kavach is configured **once**, declaratively, in `kavach.config.js`. The `@kavach/vite`
plugin reads that file and generates the `$kavach/*` virtual modules — a pre-configured
server instance, the instantiated adapter, and the provider list. You never hand-wire the
adapter or call `createKavach` in `hooks.server`; you import the ready instance and let its
`handle` hook run. Break that indirection and the app stops reconfiguring from one file.

```
kavach.config.js  ──▶  @kavach/vite  ──▶  $kavach/auth      (kavach, adapter, logger)
   adapter               (plugin,           $kavach/providers (generated provider list)
   providers[]            before             $kavach/config    (normalized config)
   routes{}               sveltekit())       $kavach/routes    (route map)
   rules[]                                        │
   env{}                                          ▼
                                     hooks.server.js  export const handle = kavach.handle
                                     +layout.svelte   createKavach(adapter, {...}) (browser)
```

**Fastest correct path:** run the CLI. It writes every file below and can repair a broken setup.

```bash
npx kavach init         # scaffolds config, patches vite/hooks/layout, generates the auth page
npx kavach doctor --fix # diagnoses and repairs an existing setup
```

The rest of this skill is what `init` produces — read it to author or review a setup by hand.

---

## 1. Install

```bash
npm install kavach @kavach/vite
npm install @kavach/adapter-supabase   # pick ONE adapter — required (firebase/auth0/amplify/convex)
```

The core package is named **`kavach`** (unscoped). It has **no default export**.

---

## 2. kavach.config.js

A single default-exported object. Only `adapter` and at least one provider are required;
everything else has a default.

```js
// kavach.config.js
export default {
  adapter: 'supabase',
  providers: [
    { name: 'google', label: 'Continue with Google' },
    { name: 'magic', mode: 'otp', label: 'Email Magic Link' }
  ],
  routes: {
    auth: '/auth', // sign-in page (maps to the internal `login` route)
    home: '/dashboard', // post-login landing — string OR async (session) => path
    logout: '/logout',
    data: '/data' // omit to disable the data endpoint
  },
  rules: [
    // route protection — see the kavach-authorization skill
    { path: '/', public: true },
    { path: '/auth', public: true },
    { path: '/dashboard', roles: '*' },
    { path: '/admin', roles: ['admin'] }
  ],
  env: {
    url: 'PUBLIC_SUPABASE_URL',
    anonKey: 'PUBLIC_SUPABASE_ANON_KEY'
  }
}
```

| Key            | Purpose                                                                     |
| -------------- | --------------------------------------------------------------------------- |
| `adapter`      | Backend adapter name (`supabase`, `firebase`, `auth0`, `amplify`, `convex`) |
| `providers[]`  | Sign-in methods — see the **kavach-providers** skill                        |
| `routes{}`     | Auth/endpoint paths (table below)                                           |
| `rules[]`      | Declarative route protection — see the **kavach-authorization** skill       |
| `env{}`        | Maps adapter config to your `PUBLIC_*` env var names                        |
| `logging{}`    | `{ level, table \| collection \| entity }` audit sink                       |
| `cachedLogins` | Remember recent logins for the sign-in UI                                   |

---

## 3. Changing the paths (`routes`)

`routes` is where you move any auth path. Defaults are applied by `@kavach/vite`; the ones
you omit fall back to these, and the internal engine (`@kavach/sentry`) resolves the rest.

| `routes` key | Default         | What it is                                                                                 |
| ------------ | --------------- | ------------------------------------------------------------------------------------------ |
| `auth`       | `/auth`         | Sign-in page. Internally this becomes the `login` route (401 redirects go here).           |
| `logout`     | `/logout`       | `kavach.handle` serves this itself: signs out, clears the cookie, 303-redirects to `auth`. |
| `home`       | `/`             | Post-login landing. String **or** `async (session) => path` resolver (per-role landing).   |
| `session`    | `/auth/session` | Endpoint `kavach.handle` uses to sync the session cookie. Treated as an endpoint route.    |
| `data`       | `/data`         | Data CRUD endpoint served by `handle` — see **kavach-data-access**. Set `null` to disable. |
| `rpc`        | `/rpc`          | RPC endpoint served by `handle`. Set `null` to disable.                                    |

To relocate the login page to `/signin`, change **one** line — `routes.auth: '/signin'` —
and update the matching `rules[]` entry. Do **not** hardcode `/auth` anywhere else; read it
from `$kavach/routes` if a component needs it.

```js
// per-role landing without any load-function redirects:
routes: {
  home: async (session) =>
    session.user.role === 'admin' ? '/platform' : `/${session.user.user_metadata.slug}`
}
// Receives the full session; must return a path string. If it throws, kavach falls back to '/'.
```

---

## 4. vite.config.js — plugin order matters

```js
import { kavach } from '@kavach/vite'
import { sveltekit } from '@sveltejs/kit/vite'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [kavach(), sveltekit()] // kavach() MUST come before sveltekit()
})
```

`kavach()` generates the `$kavach/*` virtual modules that `hooks.server` and the client import.
Without it, `$kavach/auth` fails to resolve.

---

## 5. hooks.server.js — one line

```js
import { kavach } from '$kavach/auth'
export const handle = kavach.handle
```

`$kavach/auth` already exports a **fully configured** server instance. `kavach.handle`:
sets `event.locals.session` from the cookie on every request, serves the `session`,
`logout`, `data` and `rpc` routes itself, and protects everything else against `rules[]`.

---

## 6. +layout.server — expose the session

```js
// src/routes/+layout.server.js
export function load({ locals }) {
  return { session: locals.session, user: locals.session?.user ?? null }
}
```

`kavach.handle` set `locals.session` for you — just pass it through. The session shape is
`{ user: { id, email, role }, access_token, refresh_token, expires_in }`.

---

## 7. Browser instance — createKavach in the root layout

The **only** place a consumer calls `createKavach` is the browser, in the root layout's
`onMount`. Share it via context so any component can call `signIn` / `signOut`.

```svelte
<!-- src/routes/+layout.svelte -->
<script>
  import { setContext, onMount } from 'svelte'
  import { page } from '$app/stores'

  const kavach = $state({})
  setContext('kavach', kavach)

  onMount(async () => {
    const { createKavach } = await import('kavach')
    const { adapter, logger } = await import('$kavach/auth')
    const { invalidateAll } = await import('$app/navigation')
    const instance = createKavach(adapter, { logger, invalidateAll })
    Object.assign(kavach, instance)
    instance.onAuthChange($page.url) // parses OAuth callbacks, syncs the session
  })
</script>
```

Downstream components read it with `getContext('kavach')` and call `kavach.signIn(...)` /
`kavach.signOut()` — never the backend SDK directly. See **kavach-providers**.

---

## 8. Virtual modules generated by @kavach/vite

| Module              | Exports                                                      | Use in                        |
| ------------------- | ------------------------------------------------------------ | ----------------------------- |
| `$kavach/auth`      | `kavach` (server instance w/ `.handle`), `adapter`, `logger` | `hooks.server`, client layout |
| `$kavach/providers` | `providers` (the normalized provider list)                   | your sign-in page             |
| `$kavach/config`    | the normalized config object                                 | rarely needed                 |
| `$kavach/routes`    | the resolved route map                                       | components that need a path   |

`@kavach/vite` also emits an ambient `src/kavach.d.ts` so the `$kavach/*` imports type-check.
Note: `svelte-kit sync` / `svelte-check` do **not** run Vite plugins — only `vite dev` / `vite build`
generate the declaration file.

---

## Common mistakes

| Mistake                                          | Why it fails                                  | Fix                                                                         |
| ------------------------------------------------ | --------------------------------------------- | --------------------------------------------------------------------------- |
| `import kavach from 'kavach'`                    | No default export → `undefined`               | Named import from `$kavach/auth`: `import { kavach } from '$kavach/auth'`   |
| `createKavach(adapter).handle` in `hooks.server` | Re-configures a second, adapter-less instance | `$kavach/auth` already has the configured instance — use `kavach.handle`    |
| `sveltekit()` before `kavach()`                  | Virtual modules not registered in time        | `plugins: [kavach(), sveltekit()]`                                          |
| `resolve.alias` / `ssr.noExternal: ['kavach']`   | Fights the virtual module resolution          | Remove them; use `$kavach/auth`                                             |
| Hardcoding `/auth`, `/logout` across the app     | Drifts from `routes` config                   | Change `routes` once; read paths from `$kavach/routes`                      |
| Calling `createKavach` on the server             | The server instance is generated for you      | Only call it in the browser layout `onMount`                                |
| `$kavach/auth is not a module` in `svelte-check` | `check` doesn't run Vite plugins              | Run `vite dev`/`build` once to emit `kavach.d.ts`, or `kavach doctor --fix` |

When a setup is broken, `npx kavach doctor` names the exact failure and `--fix` repairs the
common ones. Reach for it before hand-editing.
