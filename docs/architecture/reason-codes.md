---
name: Reason codes
description: One registry answering "why didn't this happen?" across sharing, schedules, governance and sync — scoped by domain, classified by kind
date: 2026-08-28
---

# Reason codes — one registry, scoped by domain

## The question

"Why didn't this happen?" is asked of at least five things already:

- a repository is not syncing
- a scheduled worker has not run
- metrics were not pushed
- governance was not pulled
- a rule pack was not adopted

Each currently answers in its own way — a boolean, a free-text `last_error`, a
log line, or nothing at all. So the same question has five shapes, and a UI that
wants to explain any of them re-implements the vocabulary each time.

## Not one registry per segment, and not one flat registry

**Per segment** (`dojo.share_reasons`, `sensei.schedule_reasons`, …) duplicates
the shape five times: five tables, five joins, five UI components, five places to
forget `remedy`. The second copy is already a smell; the fifth is a guarantee of
drift.

**One flat registry** loses distinctions that matter. Two in particular:

1. **Precedence is only meaningful WITHIN a domain.** "Fix the subscription before
   the election" is a real ordering; "fix the subscription before the schedule
   window" is meaningless. A global ordering would be arbitrary.
2. **Not every "didn't happen" is a problem.** A schedule that is *not due yet* is
   working correctly. A subscription that lapsed is not. Flattening them means the
   UI cannot tell *fine* from *broken* — which is the exact failure this project
   keeps hitting, and which `sensei.sync_state` already solved once by
   distinguishing `skipped` (deliberate) from `error` (fault).

## The shape: one table, two scoping axes

```
sensei.reason_codes
  domain      text        -- 'repository_sharing' | 'schedule' | 'metric_push'
                          -- | 'governance_pull' | 'rule_pack_adoption'
  code        text        -- 'not_subscribed', 'not_due', 'session_rejected'
  kind        sensei.reason_kind   -- 'normal' | 'refusal' | 'fault'
  precedence  smallint not null    -- ordered WITHIN a domain; lower = fix first
  summary     text not null        -- one line, for the row
  detail      text not null        -- the paragraph behind a tooltip
  remedy      text                 -- what to DO; NULL when nothing can be done
  actor       text                 -- who can act; NULL when nobody
  primary key (domain, code)
  unique (domain, precedence)
```

### `kind` is the axis that earns the single table

| kind | meaning | UI treatment |
|---|---|---|
| `normal` | nothing is wrong | plain text, no alarm |
| `refusal` | a deliberate decision by someone | show WHO and the remedy |
| `fault` | something broke | surface it; it needs attention |

This maps onto a distinction the project already made:
`sync_state.state ∈ (synced, error, skipped)` — where `skipped` exists precisely
because *"a private repository is not a sync failure, it is a choice"*. `kind`
generalises that to every domain instead of re-deciding it per feature.

Concretely, three reasons a scheduled worker did not run:

| code | kind | why the distinction matters |
|---|---|---|
| `not_due` | normal | it ran 20 minutes ago; nothing to see |
| `disabled` | refusal | someone turned it off — say who, offer to turn it on |
| `outside_window` | normal | it is 3pm and the window is 22:00–05:00 |
| `worker_failed` | fault | it tried and threw; this needs a human |

A dashboard that treats all four as "not running" cries wolf on two of them and
stays silent on the one that matters.

## Where it lives, and how it reaches both planes

`sensei` schema, because both planes need it: the daemon answers schedule and push
questions, the dōjō answers sharing and entitlement ones.

`sensei` is **not** wholly included in the `dojo` scope — that scope lists
`sensei.*` objects explicitly (see `database/design.yaml`). So the registry needs:

- an `includes:` entry for `sensei.reason_codes` and `sensei.reason_kind`
- its DATA seeded through the staging import path, exactly as `sensei.scopes`
  already is — the design.yaml comment on that spells out why: *"never a
  hand-rolled INSERT that would drift on the next reset+deploy"*

And because `sensei` is deliberately unexposed to PostgREST, the dōjō reads it
through a view — `dojo.reason_codes`, the same sanctioned pattern as
`dojo.metric_catalogue`.

## The boundary: the registry REPORTS, the domain DECIDES

This is the rule that keeps the registry useful, and it is the one that will be
under pressure the first time someone wants to add a column.

| question | answered by | where it lives |
|---|---|---|
| *which reason applies right now?* | **the domain** | the view, or the enforcing procedure |
| *given that code, what do I show a human?* | **the registry** | `sensei.reason_codes` |

The registry is **reporting data**. It holds no predicate, no condition, no
threshold, and no branch. `dojo.all_my_repositories` decides that a repository is
`not_subscribed`; the registry only knows what those words mean to a reader and
who can act on them.

### The test

**Delete the entire registry and the system must behave identically** — same
repositories syncing, same schedules running, same rows pushed. The only loss is
that a UI renders `not_subscribed` instead of "No active subscription for this
organization".

If deleting it would change behaviour, logic has leaked in.

### The columns that must never exist

`condition` · `predicate` · `when` · `sql` · `expression` · `threshold` ·
`applies_if`

A request for any of these is the signal that someone is trying to move a
decision out of the domain and into a lookup table. The result is a rule split
across SQL and data, where neither half is readable alone and nobody can say
where the decision was made. That is worse than the four duplicated derivations
this registry exists to remove, because at least those were each readable.

### The subtle case: `precedence` orders DISPLAY, never gates BEHAVIOUR

`precedence` is the one column that could be mistaken for logic, so the line is
worth stating: the domain decides which codes APPLY; precedence decides which of
them to SHOW first. A repository failing three ways is failing three ways
regardless of what the registry says.

So `sync_enabled` — and every other behavioural verdict — is computed **entirely
from domain data**, never from a join against the registry. The registry is
joined only to decorate a verdict already reached. Concretely, in
`all_my_repositories`:

```sql
-- CORRECT: the verdict, from domain data only
, (may_share and elected)                as sync_enabled
-- CORRECT: the registry decorates it
, rc.summary                             as reason
, rc.remedy

-- WRONG: behaviour now depends on a lookup row
, (select ... from sensei.reason_codes where ...) as sync_enabled
```

An enum in the registry that no domain emits is dead copy, and a code a domain
emits that the registry lacks should surface as the raw code rather than an empty
string — a missing translation must degrade to something readable, never to
silence.

## What a consumer gets

A view or endpoint joins on `(domain, code)` and returns `summary`, `detail`,
`remedy`, `actor`, `kind`. No consumer writes copy, and adding a reason is a seed
row rather than a code change in every UI that renders it.

`dojo.all_my_repositories` uses `domain = 'repository_sharing'`;
`GET /api/tasks/scheduled` uses `domain = 'schedule'`. Same join, same columns,
same component.

## The rule this generalises

For anything configurable or scheduled, three things belong together:

1. **the setting as a row**, never a literal
2. **a listing** showing configured and default side by side
3. **a registered reason** for the current state, with a remedy and an actor

`sensei.schedules` did (1) and (2). This adds (3), and makes it reusable rather
than a fourth bespoke vocabulary.
