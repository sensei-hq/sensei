---
name: Observability
description: The four views that answer what the indexer produced and what it failed to produce
date: 2026-09-23
status: current
---

# Stage 19 — observability

Four database objects answer "what did the indexer produce, and what did it fail
to produce" without anyone hand-writing a join. Two describe the **graph**, two
describe the **jobs that built it**.

| object | grain | answers |
|---|---|---|
| `sensei.graph_nodes` | one node | how many nodes, in which repository, language, kind, locality |
| `sensei.graph_resolution` | one placed edge | how each edge was derived — the `via` rung |
| `activity.task_health` | one execution | what ran, where, how long, and why it failed |
| `activity.task_failures` | one broken job | what is STILL broken — the restart list |

`sensei.error_signature(text)` supports the last two: it collapses an error
message to its shape so failures group by cause instead of by file.

---

## §1 The graph half

Both views now carry `repository_id` / `repository`, so "how many nodes belong
to which repository" is a `GROUP BY` rather than a join the caller re-derives:

```sql
select repository, language, count(*) from sensei.graph_nodes
 where repository is not null group by 1, 2 order by 3 desc;
```

Attribution is a **direct read of `folders.repository_id`**, never
`sensei.repo_anchor_for()`. That resolver walks a path to its nearest ancestor
anchor, which is right for an arbitrary cwd and wrong here — a node already
carries a `folder_id`, and rolling it up to an ancestor would report it against
a folder it never touched. A folder with no repository reads NULL, which is the
query that finds what still needs attributing.

**Count at node grain.** 444,658 of 467,707 nodes (95.1%) resolve to a
repository. The folder-grain figure is 1.4% and is the wrong denominator:
12,371 of 13,035 folders are plain directories holding no nodes, while 183 of
193 `git` folders are attributed.

## §2 `via` is structurally empty until the cutover completes

`graph_resolution.resolved_via` reads `edges.props->>'rung'`. Measured
2026-09-23: **zero of 965,518 edges carry that key** — `edges.props` holds only
`receiver_return_of` and `relation`.

This is not a defect in the view. The only writer of `rung` is
`indexer/persist.rs`, which is **v2**. Every production edge was written by v1,
which never emitted one. The column fills in per language as each adapter enters
`PRODUCTION_LANGUAGES` (see [`13-cutover.md`](13-cutover.md)), starting with
rust.

The same split explains the node-kind population. The vocabulary in
[`17-vocabulary.md`](17-vocabulary.md) §2 describes v2's kinds; production holds
v1's. `EnumVariant`, `Field`, `Property`, `Trait`, `Macro` and `Static` have
**zero rows** today, while `section`, `doc`, `rationale` and `package` — which
the vocabulary does not list — are present in volume. Read any aggregate over
`kind` as "what v1 produced" until the cutover lands.

## §3 The job half

`task_health` is every execution; `task_failures` is the subset still broken.

A job is keyed `(task_kind, folder_path, path)` — exactly the triple a task is
re-enqueued from, which is what makes the second view a restart list. It keeps a
job only when its **most recent** execution failed: one that failed and later
succeeded is finished work, and restarting it would redo it.

### Both scope columns are overloaded, and the views say so

`folder_path` is an absolute path for folder-scoped kinds and a **UUID** for
group-scoped ones (`compute_group_metrics`). `path` is a file for
`process_file` and a **metric name** (`churn`, `duplication`, `quality`) for the
metric kinds.

So the folder join is guarded on `folder_path like '/%'` rather than left to
miss, and `is_folder_scoped` separates the two populations. A NULL repository on
a group-scoped row is CORRECT, not an unattributed gap — an unguarded join would
make it look like a folder-scoped task whose folder had vanished, which is a bug
report about nothing.

`error_signature` collapses paths but deliberately **keeps** fqns, languages and
metric names: a duplicate-identity failure in rust and one in typescript are
different problems, and merging them would hide both.

### `attempts` vs `max_retry` — the poison-pill signal

`max_retry` is the runner's counter on the last execution. `attempts` is how
many failures the job accumulated. They diverge when a job is re-enqueued fresh
each pass, and that divergence is the diagnostic: **high `attempts`, flat
`max_retry` means the job is being rediscovered and re-failed, not retried.**

```sql
select repository, task_kind, path, attempts, max_retry
  from activity.task_failures where attempts > 10 order by attempts desc;
```

Live on 2026-09-23 this returns `detect_communities` at **415 attempts against
max_retry 3** — the shape, in one row.

## §4 Why a lifetime total is not a status

The tree held 19,807,261 executions of which 17,634,822 were failures (89%),
from the stage-3 barrier gap (see [`05-structure-write.md`](05-structure-write.md)).
A cumulative count **carries no tense**, and reading that total as a live
incident was wrong: an hourly breakdown showed failures going 79,681 → 2 → 0 the
hour the fix deployed.

So the first query against a failure count is always the time axis:

```sql
select date_trunc('hour', started_at) as hr,
       count(*) filter (where status = 'failed')    as failed,
       count(*) filter (where status = 'completed') as completed
  from activity.task_health where task_kind = 'process_file'
 group by 1 order by 1 desc limit 24;
```

The lake was rolled into `activity.task_execution_daily` and reclaimed on
2026-09-23 — 17,577,049 rows, 14 GB → 656 MB. What survives is the daily counts
plus every failure with a distinct cause.

## §5 Retention, and why the lake outlived its window

`tasks/activity_pruner.rs` keeps successful executions 14 days and **failed ones
90** — on the stated premise that failures are "rare enough to be free (32,664
of 4.8M — 0.7%)". That premise does not hold against a poison-pill loop: the
lake was 89% of the table and 84 days old, so it sat inside the window by
design.

The rollup is also bounded by the **success** retention (14 days), so the lake's
own days had never been aggregated — deleting it without rolling up first would
have destroyed the counts the retention policy exists to preserve.

> **Open:** failure retention has no cap on repeated identical failures. Nothing
> prevents this refilling. The fix is to keep N exemplars per (kind, signature,
> day) rather than every row; it is not implemented.
