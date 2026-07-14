# 綜 · Solution · Sessions

**Segment:** 04 · Project window — solution variant
**Route:** `/solution/[id]/sessions`
**Source mockup:** [`lib/observatory/solution-track.jsx`](../../mockups/Sensei/lib/observatory/solution-track.jsx) → `SolutionSessions` (solution-scoped session digest)
**App file:** `app/src/routes/solution/[id]/sessions/+page.svelte`

## Purpose

Sessions across every project in the solution. Same primitive
as [[screen/project-sessions]] scoped to the solution's member
projects.

## Data invariants

- `GET /api/sessions?solution=<id>&range=…` — same shape as
  [[screen/project-sessions]] plus each row carries the member
  `project_name` and its `role` in the solution.

## Signals shown

Same as [[screen/project-sessions]] plus:

- **Project filter chip strip** — narrow by member project.
- **Role facet** — filter by role (`primary` / `platform` /
  `product-team` / etc.).
- Session row: shows the member project + folder role.

## Done gate

- Every session across member projects appears; the visible
  session count equals `sum` over member projects of
  `count(*) from activity.sessions where project_id = {id_i}
  and started_at within range`.
- Filter chips narrow correctly; totals row respects the
  filter (e.g. selecting only `role = ui` matches
  `count from sessions where folder_role = 'ui' and project_id
  in {member_ids}`).
- Row navigates to Replay scoped to the underlying session.

Optional check:
```
curl -s "http://localhost:7744/api/sessions?solution={id}&range=7d" \
  | jq '{n: length, by_role: (group_by(.folder_role) | map({role: .[0].folder_role, n: length}))}'
```

## Wrong gate

- **Only primary project's sessions appear.** Aggregation not
  running.
- Every failure mode inherited from
  [[screen/project-sessions]] applies.

## Related

- [[screen/project-sessions]] — single-project peer
- [[screen/solution-dashboard]] — parent
