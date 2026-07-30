# 群 · Observatory · Collective

**Segment:** 03 · Observatory — daily use
**Route:** `/dojo/sharing` (sharing prefs) — this screen is the collective-intel controls
**Source mockup:** [`lib/observatory/collective-settings.jsx`](../../mockups/Sensei/lib/observatory/collective-settings.jsx) → `ObsCollectiveSettings`
**App file:** `app/src/routes/(observatory)/dojo/sharing/+page.svelte`

## Purpose

Collective is the controls for what leaves the user's machine
and toward whom. The mental model simplifies now that the global
Collective is modelled as a special-case Dōjō (`global-dojo`) —
every destination is a Dōjō, just at different scopes.

The user decides:

- **Mode** — share to global-dojo (public commons), to a company
  Dōjō, to both, to neither.
- **Cadence** — how often batches are prepared.
- **Per-category filters** — memory · pattern · rule · prompt ·
  guard · skill · agent.

The destinations coexist. Personal Sensei can run without either
being enabled.

Kanji is 群 — *collective*.

## Data invariants

- `GET /api/preferences/collective` returns the current mode,
  cadence, and per-category on/off + attribution defaults.
- `PUT /api/preferences/collective/{key}` updates.
- Credit default for personal-closed-source work: `named` when
  going to a Dōjō, `anonymous` when going to the global collective
  (default; user can override for their own OSS work). Source-
  dereference is always-on regardless of credit.

## Signals shown

| Element | Value |
|---|---|
| Destination toggle | 4 states: none · global · dojo · both |
| Cadence chip strip | daily / weekly / manual |
| Category toggle grid | 6 rows × 2 columns (global on/off, dojo on/off) |
| Credit defaults | small block per destination: `named / anonymous` |
| Preview block | "next batch" preview when non-manual cadence |

## Done gate

- Every write persists and takes effect immediately (the next
  batch honors the new setting).
- The two destinations are independent — enabling Dōjō doesn't
  side-effect global; disabling one doesn't affect the other.
- With both toggles off, `/api/share-review/next-batch` returns
  an empty items array.
- Category toggles are respected by the loop
  ([[pipeline/dojo-lifecycle]] contribute step) — turning
  `memory` off drops memory items from the next batch and only
  memory items.
- Attribution defaults per destination match the rules in
  [[pipeline/dojo-lifecycle]] attribution table.
- Manual cadence pauses batching until the user clicks the
  next-batch trigger.

Optional check:
```
curl -s http://localhost:7744/api/preferences/collective | jq
# then flip a toggle:
curl -X PUT http://localhost:7744/api/preferences/collective/global_memory \
     -H 'Content-Type: application/json' -d 'false'
curl -s http://localhost:7744/api/share-review/next-batch \
  | jq '[.items[] | select(.type=="memory" and .destination=="collective")] | length'
# expected: 0
```

## Wrong gate

- **Global and Dojo controls share state.** Journey map fix
  reverted.
- **Category off but items still ship.** Filter not applied at
  the contribute step.
- **Client-work item appears in the global preview without
  dereference.** Confidentiality regression.
- **Cadence chip shows daily but batches never fire.** Scheduler
  not honoring cadence.

## Related

- [[pipeline/dojo-lifecycle]] — the loop this configures
- [[pipeline/governance]] — attribution rules
- [[screen/observatory-upgrades]] — downstream peer
- [[screen/observatory-share-review]] — the batch review surface
