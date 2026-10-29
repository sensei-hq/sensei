# Checkpoint — 2026-10-29 (transcripts count automated turns; Diagrams nav)

**Slice:** the user's 2026-10-29 list — transcript turn origins ("before diagrams"), then the
Diagrams rework (mockup: sidebar subsection, per-diagram routes, levels, inspector), Nav first.

**Done (develop, pushed):**
- c9adada48 — automated prompts (scheduled / peer / task_notification / auto_continuation) are
  transcript turns with `origin` + `prompt_id`; session turns, corrections and abandonment count
  yours only; hook turns take their origin by prompt id (`activity.turns.origin`); hourly charts
  count your turns, tokens count all; a transcript turn newer than the analysis re-queues the
  session; Claude transcripts re-read once (`transcripts.turn_origins_v1`). Live DB reconciled.
- e6da28452 — Diagrams slice 1: sidebar subsection (8 entries), one `diagrams/[lens]` route on a
  shared CodeMapState; old tabs and Layers/Neighbourhood/Structure/Cycles/codemap pages retired.
- cb8087e86 — transcript turns landing make a workspace due (analyzer read session start/end only).
- 7b9a872c2 — a lens switch kept minting a new, unread CodeMapState (page.params is replaced per
  navigation); map now kept by id value. Live inspect: every lens draws on sensei, 0 errors.
- Live: 812 automated transcript turns landed; hook turns labelled by prompt id (only id-less
  prompts stay unlabelled, counted as yours). sensei 10/24 = 72 /loop ticks refused by the weekly
  limit — 0 of your turns, 0 tokens, both correct. Installed (service + app).

**Remaining (Diagrams):** 2 shared collapsible three-line header + 300px inspector at every level
+ legend/budget; 3 single-click drill, search, Communities grouping, Show at L0, band filters,
Fit/100%; 4 Database · ER inspector and mobile lists.

**Next command:** Diagrams slice 2 — start from `app/src/routes/(workspace)/workspace/[id]/diagrams/[lens]/+page.svelte`
(shared header) and `packages/codemap/CodeMap.svelte` (inspector).

**After Diagrams (user, 2026-10-29):** #347 per-repository attribution across all transcript
adapters (a session's cross-repo work counts against its start repo today); then adapters #348
Muse, #349 Freebuff.

**rokkit:** d9cbf302 (dark paper-edge at 200) on develop, unreleased — user cuts 1.12.2, then
bump app/dōjō/website.

**Open:** title-bar "scribble" (user screenshot next run); website split #346; #282 part 3
(Permissions step); one zz-a11y axe eval timeout that did not recur on rerun (cause unknown).

**Known broken:** Keychain test on rebuilt binaries (#282). sensei MCP checkpoints 404 in this
long session (stale sensei-mcp).
