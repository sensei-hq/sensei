---
name: kavach-data-access
description: Use when reading/writing data or calling RPC through Kavach — the routes.data and routes.rpc endpoints that kavach.handle serves internally (GET/POST/PUT/PATCH/DELETE), the query-param grammar (:select / :order / :limit / :offset / :count / filters), schema-qualified schema/entity paths, session-gated access (401 when unauthenticated), and the advanced +server.ts override that re-exports the handlers from kavach — instead of hand-writing +server endpoints.
---

# Kavach Data Access — Data & RPC Endpoints

When `routes.data` (and/or `routes.rpc`) is set in `kavach.config.js`, `kavach.handle`
**serves those routes itself**. You do not create `+server.ts` files for CRUD — the handle
hook intercepts the path, checks the session, and delegates to the adapter. Requests without
an active session get `{ error: { message } }` with status `401`.

```
fetch('/data/public/users?:select=id,email&:limit=20')
        │
        ▼
kavach.handle ─▶ session present? ──no──▶ 401 { error }
        │yes
        ▼  adapter data operation (locals.kavach.actions)
     rows / result JSON
```

---

## 1. Enable the endpoints

```js
// kavach.config.js
export default {
  routes: {
    data: '/data', // kavach.handle serves GET/POST/PUT/PATCH/DELETE here
    rpc: '/rpc' // kavach.handle serves POST here
  }
}
```

Set either to `null` (or omit) to disable. Both are treated as **endpoint routes**: a denied
request returns a bare status (no redirect), which is what a `fetch` client wants.

Protect them with `rules[]` like any route (see **kavach-authorization**):

```js
rules: [
  { path: '/data', roles: '*' }, // any authenticated user
  { path: '/data/admin-stats', roles: ['admin'] } // stricter sub-path
]
```

---

## 2. Reading data (GET)

Query the entity by path; shape the result with reserved `:`-prefixed params. Everything else
is treated as a filter condition.

```js
// any authenticated fetch, e.g. from a load function or component
const res = await fetch('/data/public/users?:select=id,email&:order=email&:limit=20&active=true')
const rows = await res.json()
```

| Param                | Purpose                                        |
| -------------------- | ---------------------------------------------- |
| `:select`            | Column projection (`id,email,name`)            |
| `:order`             | Sort order                                     |
| `:limit` / `:offset` | Pagination                                     |
| `:count`             | Count mode                                     |
| _(any other key)_    | Filter condition (`active=true`, `role=admin`) |

**Schema-qualified paths** use `schema/entity`: `/data/public/users`. A bare `/data/users`
targets the adapter's default schema.

Writes use the matching verbs on the same path: `POST` (insert), `PUT`/`PATCH` (update),
`DELETE` (remove), with the body as JSON.

---

## 3. RPC

`routes.rpc` accepts `POST` with the procedure name + args in the body and returns the result.
Configure the handler through the Kavach instance's `rpc` option (the CLI/`$kavach/auth`
instance wires this from your config) — call it from the client with a plain `fetch` to
`routes.rpc`.

---

## 4. Advanced: +server.ts override

Only when you need custom logic on a data path, re-export Kavach's request handlers instead of
writing CRUD from scratch. They delegate to the adapter via `locals.kavach.actions`, which
`kavach.handle` sets for you:

```ts
// src/routes/(server)/data/[...slug]/+server.ts
export { GET, POST, PUT, PATCH, DELETE } from 'kavach'
```

`kavach.handle` sets `event.locals.kavach = { actions: dataFn }`, so the re-exported handlers
reach the same adapter operations — you get the session gating and query grammar without
reimplementing them. Add your own logic around the re-export only where the default isn't
enough.

The `data(schema, session)` function (passed when the instance is created, via config) is what
backs both the built-in `/data` route and `locals.kavach.actions` — it's the single seam for
customizing what the endpoint can touch.

---

## Common mistakes

| Mistake                                                       | Why it fails                                                          | Fix                                                                                          |
| ------------------------------------------------------------- | --------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| Writing a full `+server.ts` CRUD handler                      | Reimplements session gating + query grammar `handle` already provides | Enable `routes.data`; only override with `export { GET, POST, … } from 'kavach'` when needed |
| Calling the backend SDK from a component to read data         | Skips the session-gated endpoint and adapter mapping                  | `fetch(routes.data + '/schema/entity')`                                                      |
| Expecting `/data` denial to redirect to `/auth`               | Endpoint routes return status only (no redirect)                      | Handle the `401` in the caller                                                               |
| Filtering with a `:`-prefixed custom key                      | `:`-prefixed keys are reserved (select/order/limit/offset/count)      | Use a plain key: `active=true`, not `:active=true`                                           |
| Forgetting a `rules[]` entry for `/data`                      | Falls under default protection or leaks                               | Add `{ path: '/data', roles: '*' }` (or stricter)                                            |
| Re-exporting handlers but not reading `locals.kavach.actions` | Custom handler has no adapter access                                  | Let `kavach.handle` run (it sets `locals.kavach`); re-export from `kavach`                   |

See **kavach-setup** for enabling routes and **kavach-authorization** for protecting them.
