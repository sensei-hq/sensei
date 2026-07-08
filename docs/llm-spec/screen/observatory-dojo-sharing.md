# 送 · Observatory · Dōjō sharing — per-membership overrides

**Segment:** 03 · Observatory — daily use
**Route:** `/dojo/sharing/{membership_id}` — per-membership override view
(the `/dojo/sharing` root is owned by [[screen/observatory-collective]] for global controls; this screen is the drill-in for one membership's overrides)
**Source mockup:** [`lib/collective-settings.jsx`](../../mockups/Sensei/lib/collective-settings.jsx) → `ObsCollectiveSettings` in per-membership scope (the same primitive with the membership picker set to a specific row)
**App file:** `app/src/routes/(observatory)/dojo/sharing/[membership_id]/+page.svelte`

## Purpose

Per-Dōjō sharing controls. Similar to
[[screen/observatory-collective]] but scoped to a specific
membership. Overrides the global collective settings for items
routing to this Dōjō.

Kanji is 送 — *to send*.

## Data invariants

- `GET /api/dojo/sharing?membership_id=…` returns per-membership
  sharing preferences.
- Overrides the global settings when a value is set.
- Client memberships have a locked `dereferenced` attribution
  that cannot be changed.

## Signals shown

Same primitive shape as [[screen/observatory-collective]] but
per-membership:

- Category grid (memory / pattern / rule / prompt / guard / skill
  / agent) with on/off per membership.
- Cadence chip (daily / weekly / manual) per membership.
- Attribution default per membership (`named` / `named + internal`
  / `dereferenced` — client is locked).

## Done gate

- Each membership has its own sharing preferences page,
  reachable from a chip strip / rail.
- Overrides for a specific membership take effect immediately
  and are respected by the loop.
- Client-membership dereference is locked and shown as such.
- Reverting a specific override falls back to the global
  collective setting.

## Wrong gate

- **Client attribution can be changed from `dereferenced`.**
  Confidentiality gate violated.
- **Membership override doesn't take effect on next batch.**
- **Reverting override to default doesn't inherit from global.**

## Related

- [[pipeline/dojo-lifecycle]] — the loop this configures per-org
- [[screen/observatory-collective]] — global peer
- [[screen/observatory-dojo-connections]] — where memberships
  are managed
