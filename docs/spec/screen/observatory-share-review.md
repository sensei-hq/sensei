# 送 · Observatory · Share review

**Segment:** 03 · Observatory — daily use
**Route:** `/share-review`
**Source mockup:** [`lib/dojo/dojo-inapp.jsx`](../../mockups/Sensei/lib/dojo/dojo-inapp.jsx) → `InappShare`
**App file:** `app/src/routes/(observatory)/share-review/+page.svelte`

## Purpose

Share review is the batch-review surface. Before an upstream
batch fires (Dōjō / global) the user sees exactly what's about
to leave, scoped and attributed. Approve, adjust attribution,
edit the payload, or hold back individual items.

Nothing leaves without passing through here when cadence is
`daily` or `weekly`; `manual` cadence gates every batch here
explicitly.

Kanji is 送 — *to send*.

## Data invariants

- `GET /api/share-review/next-batch` returns items queued for the
  next batch:
  ```json
  { "cadence": "daily|weekly|manual",
    "next_batch_at": iso,
    "destination": "dojo:{org}|collective|both",
    "items": [
      { "id": "…", "type": "memory|pattern|rule|prompt|guard|skill|agent",
        "title": "…", "body": "…", "scope": {…},
        "attribution": { "author": "…", "org": "…", "will_dereference": bool },
        "state": "queued|held|edited" }, …
    ] }
  ```
- Held items don't ship this batch; they re-queue for the next.
- Edited items ship with the user's edit applied.
- Attribution rules from [[pipeline/dojo-lifecycle]] apply — the
  user cannot override a client-work dereference (mandatory
  strip).

## Signals shown

| Element | Value |
|---|---|
| Batch header | destination + cadence + next-batch-at + item count |
| Trigger-now button | fires immediately when cadence is `manual` |
| Item row | type kanji · title · body preview · scope chip · attribution chip |
| Attribution chip variants | `named` / `named + internal` / `dereferenced` |
| Actions per item | Hold · Edit · Send now |
| Bulk actions | Hold all · Send all · Filter by type |

## Done gate

- Every item queued for the next batch appears here; the visible
  count matches `select count(*) from dojo.upstream_queue where
  state = 'queued'`.
- Attribution shows `will_dereference: true` for client work with
  no override; the API rejects a POST that tries to override.
- Trigger-now on manual cadence sends immediately; batches over
  10 items require a confirmation dialog.
- Held items persist across daemon restart until manually
  released.
- Edit updates the payload without dropping attribution rules.

Optional check:
```
curl -s http://localhost:7744/api/share-review/next-batch \
  | jq '{destination, cadence, n_queued: (.items | length),
         n_held: [.items[] | select(.state=="held")] | length}'
```

## Wrong gate

- **A client-work item allows the user to un-dereference it.**
  Mandatory strip violated.
- **Hold action doesn't persist.** Item ships anyway.
- **Send-now fires without the user confirming for a
  large batch.** Add a confirmation for batches over N items.
- **Batch header shows next-batch-at in the past.** Scheduler
  didn't advance.

## Related

- [[pipeline/dojo-lifecycle]] — the loop upstream
- [[screen/observatory-collective]] — cadence + destination
  controls
- [[screen/observatory-upgrades]] — downstream peer
