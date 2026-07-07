# 刻 · Project window · Sessions

**Segment:** 04 · The project window
**Route:** `/project/[id]/sessions`
**Source mockup:** [`lib/project-pages.jsx`](../../mockups/Sensei/lib/project-pages.jsx) → sessions pane; shares primitives with [`lib/sessions-zen.jsx`](../../mockups/Sensei/lib/sessions-zen.jsx)
**App file:** `app/src/routes/project/[id]/sessions/+page.svelte`

## Purpose

The sessions view scoped to a single project. Same primitives as
[[screen/observatory-sessions]] (trend / stream / constellation /
bands / pulse variants + session rows) but filtered to this
project. Multi-repo projects show a folder-role chip on each
session row.

Kanji is 刻 — *moment / cut*.

## Data invariants

- `GET /api/sessions?project=<id>&range=…` returns the scoped
  session list.
- Same shape as [[screen/observatory-sessions]] response with
  the `folder_role` field populated for multi-repo projects.

## Signals shown

Same as [[screen/observatory-sessions]]:

- Range chips (7d / 30d / 90d / all)
- Totals row (count · projects · median duration) — but with
  the multi-repo split by folder role when applicable
- Quality tally (good · corrected · abandoned)
- Chart variant chips + chart body
- Session rows (with folder-role chip when multi-repo)

## Done gate

- Scoping honors project id — no other project's sessions leak.
- Multi-repo: session rows carry a folder-role chip (`web` /
  `backend` / `docs`).
- Same chart variants render.
- Clicking a session opens the Replay pane
  ([[screen/observatory-instruments-replay]]) scoped to it.

## Wrong gate

- **Sessions from other projects appear.** Filter regressed.
- **Folder-role chip missing on multi-repo sessions.** Join not
  performed.
- Every failure mode inherited from
  [[screen/observatory-sessions]] applies.

## Related

- [[screen/observatory-sessions]] — the multi-project peer
- [[pipeline/capture]] — folder-role attribution
- [[pipeline/ftr]] — session FTR
- [[screen/observatory-instruments-replay]] — where session rows land
