# 綜 · Solution · Architecture

**Segment:** 04 · Project window — solution variant
**Route:** `/solution/[id]/architecture`
**Source design:** [`docs/archive/ideas/24-desktop-observatory.md`](../../archive/ideas/24-desktop-observatory.md)
**App file:** `app/src/routes/solution/[id]/architecture/+page.svelte`

## Purpose

A **cross-repo merged graph** view. When the solution has
backend + UI + docs repos, this screen shows the connections:
which API endpoints the UI calls, which docs describe which
functions, where the trust boundaries sit. Depends on
cross-project graph edges from [[pipeline/capture]] and
[[pipeline/traceability]].

## Data invariants

- `GET /api/solutions/{id}/architecture` returns:
  ```json
  {
    "nodes": [ { "id": "…", "project_id": "…", "kind": "…",
                 "name": "…", "role": "api|ui|doc|…" }, … ],
    "edges": [ { "from": "…", "to": "…", "kind": "api_call|doc_link|import|…" }, … ],
    "communities": [ … ]
  }
  ```
- Uses `sensei.project_dependencies` for import-style edges +
  extended cross-project edges (API contract sharing, doc
  references).

## Signals shown

| Element | Value |
|---|---|
| Header | solution kanji + name + `{n} projects` chip |
| Layout picker | force-directed / grouped-by-project / grouped-by-community |
| Node | code / doc / config with role chip |
| Edge | direction + kind (`api_call`, `doc_link`, `import`) |
| Focus mode | select a node → highlight adjacent nodes across projects |
| Filter | by project, role, edge-kind |
| Sidebar detail | for focused node — its callers, doc refs, drift items |

## Done gate

- Cross-project edges surface (UI project calling Backend
  project's endpoint appears as an edge).
- Focus mode highlights only the neighbours across the whole
  merged graph.
- Doc-to-code edges from [[pipeline/traceability]] render as a
  distinct edge kind.

## Wrong gate

- **Edges stop at project boundaries** — each project renders
  as an isolated blob.
- **Focus mode only highlights within-project neighbours.**
- **Doc-link edges missing.** Traceability integration not
  wired.
- **Graph too large to render.** Need a summarised view for
  large solutions (community-collapsed mode).

## Related

- [[pipeline/capture]] — cross-project edges
- [[pipeline/traceability]] — doc-to-code edges
- [[screen/solution-dashboard]] — parent
- [[archive/ideas/24-desktop-observatory]] — source design
