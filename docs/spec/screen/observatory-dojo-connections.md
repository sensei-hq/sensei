# 連 · Observatory · Dōjō connections

**Segment:** 03 · Observatory — daily use
**Route:** `/dojo/connections`
**Source mockup:** [`lib/dojo/dojo-inapp.jsx`](../../mockups/Sensei/lib/dojo/dojo-inapp.jsx) → `InappConnection`
**App file:** `app/src/routes/(observatory)/dojo/connections/+page.svelte`

## Purpose

The Dōjō connections pane lists every Dōjō the developer belongs
to — employer, client, community. Each connection shows its kind,
role, attribution, the projects bound to it, and sync status.
There is nothing to pair: signing in is joining (#236 step 4e) —
the daemon's sync creates a connection for each membership the
signed-in persona has, and the header links to sign-in
(Settings → Dōjō).

Kanji is 連 — *to connect*.

## Data invariants

- `GET /api/dojo/memberships` returns the current memberships
  (see [[pipeline/dojo-lifecycle]] shape). A failed read is an
  error with Retry, never the empty list (the empty state tells
  you to sign in). There is no `POST`: it answers 405.
- Auth methods recorded per membership: SSO (OIDC/SAML) · GitHub
  OAuth — as the dōjō lists it.
- Every membership has a `sync_status`:
  - `healthy` — last heartbeat within window
  - `stale` — no heartbeat for > 5m
  - `error` — connection failing with a specific message
  - `authenticating` — no producer yet: every row reads this
- A project's `dojo_membership_id` binds it to exactly one
  membership; the binding is editable from Project → About.

## Signals shown

| Element | Value |
|---|---|
| Membership card | kind kanji · dojo url · role · auth chip · sync status · last-heartbeat |
| Sign-in link | Settings → Dōjō, where signing in joins every membership |
| Auth method chip | sso / github |
| Connected-scopes list | the workspaces / repositories the membership follows |
| Bound projects strip | which of my projects route findings here |
| Remove / disable / re-authenticate actions | per card |

## Done gate

- Every membership row shows up-to-date sync status.
- Signing in brings in every Dōjō the persona belongs to, each
  with the `authenticated_via` the dōjō records; there is no
  pairing form and no device token.
- Removing a membership orphans bound projects; the UI prompts
  for a new binding.
- Client memberships surface with a distinct `kind: client`
  chip so client-precedence semantics are visible.

## Wrong gate

- **A membership shows `healthy` while heartbeat is > 5m stale.**
  Status derivation off.
- **A connection exists without a signed-in persona behind it.**
  Journey map §3.6 rule — no anonymous reads or writes.
- **A failed membership read renders as "no dōjō yet".** That sends
  you to sign in instead of telling you the daemon failed.
- **Removing an employer membership silently orphans projects.**
  Prompt is required.
- **Client kind not distinguished visually.** Precedence story
  invisible to user.

## Related

- [[pipeline/dojo-lifecycle]] — memberships + routing
- [[screen/preferences]] — Profile pane sub-section for pairing
- [[screen/project-about]] — where project bindings are set
