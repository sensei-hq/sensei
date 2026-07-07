# 全 · Project window · Overview

**Segment:** 04 · The project window
**Route:** `/project/[id]/overview`
**Source mockup:** [`lib/project-lite-panes.jsx`](../../mockups/Sensei/lib/project-lite-panes.jsx) → `ProjOverviewLite` (the lite variant is the wire target)
**App file:** `app/src/routes/project/[id]/overview/+page.svelte`

## Purpose

The overview is the landing pane inside a project window. It answers
three questions the user has when they open their project:

1. **What is sensei's current teaching for this project?** — the
   hero koan, generated from the top recommendation (or "all
   quiet" when none is pending).
2. **How is this pair (me + assistants) doing here?** — the FTR
   chip + three stat blocks (sessions/memories/doc-drift) with
   sub-lines that add signal.
3. **What just happened in this project?** — a short recent-
   sessions list with FTR indicators.

Every element is **project-scoped** — same primitives as the
Observatory Today screen, but filtered to this one project. When
the mockup and daemon agree that a project has zero recent
activity, the hero shows an honest early-state message, not a
"getting started" empty screen.

Kanji is 全 — *whole / overview*.

## Multi-repo membership

A project can span multiple repos (see [[pipeline/capture]]
"Multi-repo projects"). The overview should reflect that:

- **Header eyebrow** — when the project has > 1 folder, show
  a small `{n} repos` chip alongside the client label
  (`Project · Acme · 3 repos`).
- **Recent-in-this-project list** — session `s-2891` may belong to
  the `web` repo of the `acme` project; the row should say
  `s-2891 · web · 42m · first-try right`. The role label
  (`web`/`backend`/`docs`) comes from the folder-project
  membership row.
- If multi-repo was auto-suggested and pending user acceptance
  ([[pipeline/capture]]), a small banner offers "combine with
  {other-project}?" instead of showing the merged view. Never
  merge silently.

## Data invariants

- `GET /api/projects/{id}/overview` returns:
  ```json
  {
    "project": {
      "id": "…", "name": "…", "kanji": "…", "client": "…"?,
      "vision": "…"?, "ftr": 0.0..1.0, "warn": bool,
      "sessions7d": N, "folders": [
        { "id": "…", "name": "web", "role": "web", "primary": true }, …
      ]
    },
    "top_recommendation": {
      "id": "…", "title": "…", "why": "…",
      "evidence": ["s-2891", "s-2889"], "defaultAcp": "claude-code"
    } | null,
    "stats": {
      "sessions_7d": N, "sessions_7d_corrected": N,
      "memories": { "total": N, "ready_to_share": N, "to_merge": N },
      "doc_drift": { "open": N, "referenced_docs": N }
    },
    "recentSessions": [
      { "id": "…", "title": "…", "duration": "…", "corrections": N, "ftr": bool, "time": "…", "role": "web"? }, …
    ]
  }
  ```
- The hero's `title` + `why` come through
  [[pipeline/insight-copy]] with `kind = project_top_rec_hero`
  when a recommendation exists; kind `project_all_quiet` when
  none. Fallback static strings for both.
- FTR chip reads from
  `sensei.project_ftr_metrics.ftr_14d` — same view as the projects
  index (single source of truth).
- Doc-drift count is the open row-count from
  `inference.drift_items` scoped to this project's folders.
- Memory counts scope to `scope_project_id == this project`. The
  `ready_to_share` count is memories whose promotion to `user`,
  `org`, or `collective` scope is queued (see
  [[pipeline/memory]] promotion ladder).

## Signals shown

### Header

| Element | Value |
|---|---|
| Kanji | `project.kanji` (large) |
| Eyebrow | `Project · {client or "internal"}` + optional `{n} repos` chip |
| Title | `project.name` |
| FTR label + value | `FTR · 14d` uppercase eyebrow + `{round(ftr*100)}%` display |
| FTR tone | `text-warning` when `project.warn`; otherwise `text-ink` |

### Hero card

| Element | Value |
|---|---|
| Kanji | `聴` (listening — when a top rec exists) OR `静` (still) when quiet |
| Eyebrow | `This project · sensei speaks` |
| Headline | `top_recommendation.title` OR `"All quiet — no urgent recommendations."` |
| Body | `top_recommendation.why` OR `"Sensei is observing. The next correction or pattern will surface here."` |
| Action | `send to {defaultAcp}` when a top rec exists; hidden when quiet |
| Meta | evidence session ids joined with `·` |

Both the headline and body come from insight-copy; fallback text
above matches the mockup verbatim.

### Stat blocks (3-column grid)

| Block | Value | Sub-line |
|---|---|---|
| Sessions · 7d | `stats.sessions_7d` | `{stats.sessions_7d_corrected} corrected` |
| Memories | `stats.memories.total` | `{ready_to_share} to share · {to_merge} to merge` |
| Doc drift | `stats.doc_drift.open` | `of {referenced_docs} referenced docs` (warn tone) |

### Recent-in-this-project list

| Element | Value |
|---|---|
| Section header | `今 Recent in this project` |
| Row left | session `title` |
| Row left-sub | `{id} · {duration} · {corrections} corrections` OR `first-try right` |
| Row right | `time` |
| Row right tone | success green when `ftr === true`, muted ink otherwise |
| Rows shown | max 4 |

## Done gate

- Loading `/project/{id}/overview` on Jerry's live data renders
  the header, hero, stat blocks, and recent list with real
  numbers, not template placeholders.
- FTR chip agrees exactly with the projects-index card FTR (same
  view, same number).
- Hero renders `top_recommendation` copy from insight-copy when
  gemma4 is available; fallback text otherwise.
- Stats sub-lines carry the counts that back them (not just the
  headline number).
- Multi-repo: when > 1 folder, header carries the `{n} repos`
  chip and session rows show a `role` label.
- Recent-sessions rows are clickable — navigate into
  [[screen/observatory-instruments-replay]] scoped to that
  session id.
- Doc-drift stat renders in warning tone only when the count > 0.
- Dark mode: FTR-in-warn tone stays readable.

Optional check:
```
curl -s http://localhost:7744/api/projects/sensei/overview | jq '{
  ftr: .project.ftr,
  recs: (.top_recommendation | select(. != null) | .title),
  memories: .stats.memories,
  recent: (.recentSessions | length)
}'
# expected: ftr in [0, 1]; recent up to 4; memories.total >= 0
```

## Wrong gate

- **FTR chip on this pane differs from the projects-index card
  for the same project on the same day.** Two derivations of
  the same number — pick the view.
- **Hero shows "getting started" or "no data" instead of the
  honest all-quiet message.** Fallback template forgotten;
  insight-copy fallback path needs it.
- **Multi-repo project header hides the `{n} repos` chip.** The
  overview looks like a single-repo project; user can't tell why
  the memory count is high.
- **Session row for a multi-repo project shows only the project
  name, not the folder role.** Role join isn't happening.
- **Memories count includes archived rows.** Only active states
  count.
- **`Doc drift` sub says `of N referenced docs` where `N` is 0
  but the count is > 0.** Denominator lookup broken; either
  suppress the sub or fix the query.
- **`ready_to_share` count shows memories that are already at
  `collective` scope.** Promotion-queue vs already-promoted
  confusion.
- **Recent-in-this-project rows include sessions from OTHER
  projects.** Scope filter regressed.
- **Clicking a row lands in a Replay pane that says "no tool
  calls".** Session-id resolution regressed
  ([[pipeline/capture]] gotcha).

## Related

- [[pipeline/capture]] — multi-repo project + folder membership
- [[pipeline/ftr]] — 14d rolling FTR
- [[pipeline/memory]] — memory counts (project-scoped)
- [[pipeline/insights]] — top-recommendation source
- [[pipeline/insight-copy]] — hero headline + body
- [[pipeline/traceability]] — doc-drift counts
- [[screen/observatory-projects]] — the peer index
- [[screen/observatory-today]] — the multi-project version of the same primitives
- [[screen/project-sessions]] — deeper session-level browse
- [[screen/project-memories]] — memory list scoped here
- [[screen/observatory-instruments-replay]] — where session rows lead
