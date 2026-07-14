# 連 · Observatory · Dōjō connections

**Segment:** 03 · Observatory — daily use
**Route:** `/dojo/connections`
**Source mockup:** [`lib/dojo-inapp.jsx`](../../mockups/Sensei/lib/dojo-inapp.jsx) → `InappConnection`
**App file:** `app/src/routes/(observatory)/dojo/connections/+page.svelte`

## Purpose

The Dōjō connections pane pairs the developer's Sensei with one
or many Dōjō servers — employer, client, community. Each
connection shows authentication method, role, connected scopes,
and current sync status. Adding a connection requires an
authenticated pairing (SSO, GitHub OAuth, or device-code).

Kanji is 連 — *to connect*.

## Data invariants

- `GET /api/dojo/memberships` returns the current memberships
  (see [[pipeline/dojo-lifecycle]] shape).
- Auth methods: SSO (OIDC/SAML) · GitHub OAuth · Device-code.
- Every membership has a `sync_status`:
  - `healthy` — last heartbeat within window
  - `stale` — no heartbeat for > 5m
  - `error` — connection failing with a specific message
  - `authenticating` — mid-pair
- A project's `dojo_membership_id` binds it to exactly one
  membership; the binding is editable from Project → About.

## Signals shown

| Element | Value |
|---|---|
| Membership card | kind kanji · dojo url · role · auth chip · sync status · last-heartbeat |
| Add connection button | opens a small pane picking auth method |
| Auth method chip | sso / github / device-code |
| Connected-scopes list | teams / stacks / projects the membership follows |
| Bound projects strip | which of my projects route findings here |
| Remove / disable / re-authenticate actions | per card |

## Done gate

- Every membership row shows up-to-date sync status.
- Adding a membership succeeds through SSO, GitHub OAuth, and
  device-code — each captured as `authenticated_via`.
- Removing a membership orphans bound projects; the UI prompts
  for a new binding.
- Client memberships surface with a distinct `kind: client`
  chip so client-precedence semantics are visible.

## Wrong gate

- **A membership shows `healthy` while heartbeat is > 5m stale.**
  Status derivation off.
- **Add-connection lets the user skip authentication.** Journey
  map §3.6 rule — no anonymous reads or writes.
- **Removing an employer membership silently orphans projects.**
  Prompt is required.
- **Client kind not distinguished visually.** Precedence story
  invisible to user.

## Related

- [[pipeline/dojo-lifecycle]] — memberships + routing
- [[screen/preferences]] — Profile pane sub-section for pairing
- [[screen/project-about]] — where project bindings are set
