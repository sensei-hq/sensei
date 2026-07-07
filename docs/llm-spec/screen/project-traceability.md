# 巻 · Project window · Traceability

**Segment:** 04 · The project window
**Route:** `/project/[id]/traceability`
**Source mockup:** [`lib/traceability.jsx`](../../mockups/Sensei/lib/traceability.jsx) → `ProjTraceability` variant
**App file:** `app/src/routes/project/[id]/traceability/+page.svelte`

## Purpose

Project-scoped doc drift with **confidence scoring** highlighted
and an Expected-vs-Actual detail drawer. Same underlying pipeline
as [[screen/observatory-traceability]] but scoped and slightly
denser — the user has more context here (they know this project)
so the screen leans into detail.

Kanji is 巻 — *scroll*.

## Data invariants

- `GET /api/traceability?project=<id>` — same shape as
  [[screen/observatory-traceability]] scoped by project.
- Branch-aware — active branch's drift list; a scanned-branches
  chip strip lets the user compare (deferred until multi-branch
  UI ships, see [[pipeline/capture]] branch versioning).

## Signals shown

Same as [[screen/observatory-traceability]] plus:

| Element | Value |
|---|---|
| Doc-coverage summary | `N docs · M referenced identifiers · P still resolve` |
| Confidence scoring emphasis | prominent high/medium/low chip per row |
| Auto-fix indicator | small `resolved_auto` chip on rows sensei auto-fixed |
| Expected-vs-Actual drawer | side-by-side diff with commit context (from git-follow) |
| Branch chip strip (when > 1 scanned branch) | active + pinned branch list, click to compare |

## Done gate

- Doc-coverage summary matches the row-count math.
- Expected vs Actual diff renders both signatures with commit
  context.
- Auto-fix bar visible only for `resolved_auto` rows.
- Branch strip appears only when the project has pinned
  branches (see [[pipeline/capture]]).

## Wrong gate

- **Auto-fix chip on a row that was manually fixed.** Attribution
  wrong.
- **Diff shows expected only.** Actual signature lookup broken.
- **Branch strip renders on a single-branch project.** Feature
  leaked.
- Every failure mode inherited from
  [[screen/observatory-traceability]] applies.

## Related

- [[pipeline/traceability]] — pipeline
- [[pipeline/capture]] — branch versioning
- [[screen/observatory-traceability]] — multi-project peer
- [[screen/project-overview]] — stat consumer
