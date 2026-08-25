# `policies/` — RLS that cannot live beside its table

Applied by `dbd policies` (and as the final phase of `dbd deploy`), from
`policies/<schema>/<table>.sql`, **after** every entity exists.

## The rule

A policy belongs here if it **calls a function**. Otherwise it stays in the
table's own DDL file, next to the table it protects.

## Why the split exists

`dbd apply` orders by entity type: every table is created before any function.
A policy written inline that calls one therefore fails at deploy time, not at
review time:

```
Error: Deploy failed: table:dojo.relay_sessions failed:
  function dojo.owns_membership(uuid) does not exist
```

`dbd policies` runs after functions exist, which is the only ordering in which
such a policy can be created. That is the whole reason for this directory.

Policies that call nothing — `auth.uid()` alone, or a plain column predicate —
have no such dependency and are better kept with their table, where a reader
sees the table and the rows it exposes in one place.

## Consequence worth knowing

`dbd apply` on its own does **not** create anything in here; it needs
`--with-policies`. `dbd deploy` runs the policies phase itself, so the deploy
path is unaffected. A local database brought up with a bare `dbd apply` will
have the table but not its policy — RLS enabled with no policy denies all rows
to non-superusers, which is a loud failure rather than a silent leak.

## Current contents

| file | calls |
|------|-------|
| `dojo/relay_sessions.sql` | `dojo.owns_membership` |
| `dojo/relay_inbox.sql` | `dojo.owns_membership` |
| `dojo/relay_segments.sql` | `dojo.owns_membership` |
| `dojo/repository_metrics.sql` | `dojo.can_read_repository_metric` |
