---
name: kavach-authorization
description: Use when protecting routes or configuring role-based access and redirects in Kavach — the rules[] array (public / roles / fallback), per-role landing via the routes.home resolver, the 401→login and 403→unauthorized/home redirect mapping, prefix matching order, and where protection is enforced (kavach.handle → @kavach/sentry) instead of hand-rolled load-function guards. Also clarifies that protected:true and roleHome appear in some docs but are NOT read by the current engine.
---

# Kavach Authorization — Route Rules, Roles & Redirects

Authorization in Kavach is **declarative**. You list `rules[]` in `kavach.config.js`; on every
request `kavach.handle` asks `@kavach/sentry` whether the current session may see the path and,
if not, where to send them. You do **not** write `if (!locals.session) redirect(...)` guards in
`+page.server.js` — the rules are the single source of truth, enforced in one place.

```
request ─▶ kavach.handle ─▶ sentry.setSession(locals.session)
                          └▶ sentry.protect(pathname)
                                 { status: 200 }                    → allow
                                 { status: 401, redirect: /auth }   → not signed in
                                 { status: 403, redirect: /… }      → wrong role
                                 { status: 302, redirect: home }    → already signed in, on /auth
```

---

## 1. rules[] — one rule per route prefix

```js
// kavach.config.js
rules: [
  { path: '/', public: true }, // open
  { path: '/auth', public: true }, // open (sign-in page MUST be public)
  { path: '/dashboard', roles: '*' }, // any authenticated user
  { path: '/admin', roles: ['admin'] }, // only the 'admin' role
  { path: '/data/admin-stats', roles: ['admin'] }
]
```

A rule (`RoutingRule`):

| Field      | Default | Meaning                                                                       |
| ---------- | ------- | ----------------------------------------------------------------------------- |
| `path`     | —       | Route prefix. Matches the path itself and everything under `path/`.           |
| `public`   | `false` | `true` → open to everyone, no session needed.                                 |
| `roles`    | `'*'`   | Allowed role(s): a string, a `string[]`, or `'*'` for any authenticated user. |
| `fallback` | —       | Per-rule redirect/status override on denial (see §4).                         |

**Protection is the default.** A rule that isn't `public` requires a session; with no `roles`
it defaults to `'*'` (any authenticated user). So `{ path: '/dashboard' }` and
`{ path: '/dashboard', roles: '*' }` are equivalent — both mean "must be signed in."

The role is read from **`session.user.role`**. Set it in your adapter/backend so it lands on
the session cookie.

---

## 2. Matching order

Rules are **prefix-matched, deepest path first** — the most specific rule wins.

```js
rules: [
  { path: '/admin', roles: ['admin'] }, // /admin, /admin/anything
  { path: '/admin/audit-logs', roles: ['auditor', 'admin'] } // more specific → checked first
]
```

`kavach.handle` also auto-adds rules for its own routes: `login`/`session` are public,
`home`/`logout`/`unauthorized` require a session — you don't list those unless you want to
override them.

---

## 3. Per-role landing — the routes.home resolver

There is **no role→home map**. Per-role landing is expressed by making `routes.home` a
function that inspects the session and returns a path. It's called at redirect time whenever a
user should be sent "home" (e.g. an authenticated user hitting `/auth`, or a `302`).

```js
// kavach.config.js
routes: {
  home: async (session) => {
    if (session.user.role === 'admin') return '/platform'
    return `/${session.user.user_metadata.slug ?? 'home'}`
  }
}
```

Receives the full session; returns a path string. If it throws, Kavach falls back to `'/'`.
A plain string (`home: '/dashboard'`) is wrapped into a resolver automatically.

---

## 4. Redirect targets on denial

When `protect(path)` denies access, the status and redirect are computed as:

| Situation                                        | Status           | Redirect target                      |
| ------------------------------------------------ | ---------------- | ------------------------------------ |
| No session (unauthenticated)                     | `401`            | `routes.auth` (the `login` route)    |
| Has session, wrong role                          | `403`            | `routes.unauthorized ?? routes.home` |
| Signed in, visiting the sign-in page             | `302`            | `routes.home` (resolved per session) |
| Endpoint route (under `endpoints`, e.g. `/data`) | bare `401`/`403` | _(no redirect — status only)_        |

**Per-rule override — `fallback`:** attach `fallback` to a specific rule to override the
above for that route only:

- `fallback: <number>` → return that **status** instead (e.g. `404` to hide a route's existence).
- `fallback: '<path>'` → **redirect** to that path instead of the default target.

```js
rules: [
  { path: '/beta', roles: ['tester'], fallback: '/waitlist' }, // non-testers → /waitlist
  { path: '/secret', roles: ['admin'], fallback: 404 } // non-admins → 404, not a redirect
]
```

---

## 5. Enforcement — you don't guard routes by hand

Because `kavach.handle` runs `sentry.protect` for every non-endpoint path, protected pages
need **no** per-route auth code. Reading the session in a `load` for display is fine; using a
`load` to _enforce_ access is a smell — the rule already did it, and a hand-rolled guard drifts
from the config.

```js
// ❌ redundant — a rule already protects /dashboard
export function load({ locals }) {
  if (!locals.session) redirect(303, '/auth') // duplicates rules[], can drift
  return { user: locals.session.user }
}

// ✅ the rule enforces access; load only reads
export function load({ locals }) {
  return { user: locals.session.user }
}
```

Standalone use (`createSentry` from `@kavach/sentry`) exists for apps that want route
protection without the rest of Kavach, but inside a Kavach app the wiring is automatic.

---

## Common mistakes

| Mistake                                                       | Why it fails                                                                                                                            | Fix                                                                                               |
| ------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| `{ path: '/dashboard', protected: true }`                     | The engine does **not** read a `protected` flag; it only checks `public`. It happens to work solely because `public` defaults to false. | Use `roles: '*'` (any auth) or `roles: ['role']`. Drop `protected`.                               |
| `roleHome: { admin: '/x' }` in config                         | `roleHome` appears in older docs but is **not consumed** by the current sentry/processor.                                               | Per-role landing goes in the `routes.home` resolver (§3); per-route overrides in `fallback` (§4). |
| Forgetting `{ path: '/auth', public: true }`                  | Sign-in page requires auth → redirect loop to itself                                                                                    | Mark the auth route `public: true`.                                                               |
| Hand-rolled `if (!locals.session) redirect()` in every `load` | Duplicates `rules[]`; drifts; misses role checks                                                                                        | Add a rule; let `kavach.handle` enforce it.                                                       |
| Role rule but `session.user.role` never set                   | Every check sees role `null` → 401/403                                                                                                  | Populate `role` in the adapter so it reaches the session.                                         |
| Ordering a broad rule above a specific one                    | Rules are matched deepest-first, but overlapping same-depth prefixes can mask                                                           | Keep the specific path as its own deeper rule; verify with the demo config shape.                 |
| Expecting `/data` denial to redirect                          | Endpoint routes return a bare status, no redirect                                                                                       | That's intended for fetch clients; handle the 401/403 in the caller.                              |

See **kavach-setup** for `routes`/config wiring and **kavach-data-access** for protecting the
`/data` and `/rpc` endpoints.
