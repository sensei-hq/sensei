# 検 · Dōjō · Maintainer console

**Segment:** Dōjō (SaaS) — console (not the desktop app)
**Route:** `dojo.sensei-hq.org/{origin}/{org}/console/maintainer` (SaaS) OR the self-hosted equivalent
**Source mockup:** [`lib/dojo/dojo-maintainer.jsx`](../../mockups/Sensei/lib/dojo/dojo-maintainer.jsx) → `DojoMaintainerConsole` (panels: Triage · Candidate · Knowledge)
**Source design:** [`Sensei Dōjō Journey Map.html`](../../mockups/Sensei/Sensei%20D%C5%8Djo%20Journey%20Map.html)

## Purpose

The maintainer console is where a trusted community member (or a
company-appointed maintainer) triages the queue, evaluates
candidates, decides, sets distribution, and measures impact.

Five stages (per the journey map):

| Stage | Kanji | Purpose |
|---|---|---|
| Open the queue | 門 | See what's waiting across owned scopes |
| Evaluate | 検 | Judge quality + fit before it spreads |
| Decide | 決 | Approve / revise / decline with a clear trail |
| Set distribution | 配 | Control who receives it |
| Publish & measure | 果 | Confirm it landed and helped |

Kanji is 検 — *inspect*.

## Data invariants

- Reads from `dojo.triage_queue`, `dojo.artifacts`,
  `dojo.decisions`, `dojo.events`.
- Every action logged with `maintainer_id` and reason.

## Signals shown

| Element | Value |
|---|---|
| Queue list | filterable by scope · type · confidence |
| Candidate card | attribution · body · evidence · similarity to existing artifacts |
| Similarity chip | when the candidate near-duplicates an existing artifact — offers a merge |
| Decision row | Approve · Revise · Decline (with reason field for each) |
| Distribution scope picker | (all-org · team X · stack Y) |
| Post-publish measurement | landing telemetry: adopted / muted / pinned counts |

## Done gate

- Every triage row shows its evidence + similarity chip
  (numeric value 0..1 against nearest existing artifact).
- Approve requires a distribution scope decision — API rejects
  a POST without `distribution_scope`.
- Decline requires a reason (non-empty text field).
- Post-publish metrics flow back within 14d: `adopted_count`
  and `muted_count` populate on the row from downstream
  telemetry.
- Every action is logged in `dojo.events` — audit-log row count
  for a given maintainer equals the number of actions
  performed.
- **DDL note:** `dojo.triage_queue`, `dojo.artifacts`,
  `dojo.decisions`, `dojo.events` are new tables required by
  this screen. Not yet in `daemon/database/`.

Optional check:
```
curl -s https://dojo.sensei-hq.org/{org}/api/triage/queue?owner={maintainer_id} \
  | jq '{n_open: (.queue | length), oldest_days: (.queue | map(.age_days) | max)}'
```

## Wrong gate

- **Queue shows items from scopes the maintainer doesn't own.**
  Filter regressed.
- **Approve without setting distribution ships to all-org by
  default.** Unsafe default.
- **Duplicate approvals possible** — similarity chip ignored.
- **Metrics never populate** — landing telemetry not flowing
  back from downstream.

## Related

- [[pipeline/dojo-lifecycle]] — the loop this operates
- [[screen/dojo-admin-console]] — sibling
- [[screen/dojo-developer-flow]] — upstream source
