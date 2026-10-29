# Checkpoint — 2026-10-29 (transcripts count automated turns; Diagrams nav)

**Slice:** the user's 2026-10-29 list — transcript turn origins ("before diagrams"), then the
Diagrams rework (mockup: sidebar subsection, per-diagram routes, levels, inspector), Nav first.

**Done (develop, pushed):**
- c9adada48 — automated prompts (scheduled / peer / task_notification / auto_continuation) are
  transcript turns with `origin` + `prompt_id`; session turns, corrections and abandonment count
  yours only; hook turns take their origin by prompt id (`activity.turns.origin`); hourly charts
  count your turns, tokens count all; a transcript turn newer than the analysis re-queues the
  session; Claude transcripts re-read once (`transcripts.turn_origins_v1`). Live DB reconciled.
- e6da28452 — Diagrams slice 1: sidebar Diagrams subsection (8 entries, mark + hint), one
  `diagrams/[lens]` route over one shared CodeMapState, schema stays; map's own lens tabs gone;
  Layers / Neighbourhood / standalone Structure / Cycles / codemap pages retired; glyph table
  follows the mockup. e2e: diagrams-nav, multi-window, a11y, schema green.

**Remaining (Diagrams):** 2 shared collapsible three-line header + 300px inspector at every level
+ legend/budget; 3 single-click drill, search, Communities grouping, Show at L0, band filters,
Fit/100%; 4 Database · ER inspector and mobile lists.

**Next command:** verify live after `make install` — `psql -h localhost -p 5432 sensei -c "select
origin, count(*), sum(tokens_in) from activity.transcript_turns group by 1"`; then slice 2.

**rokkit:** d9cbf302 (dark paper-edge at 200) on develop, unreleased — user cuts 1.12.2, then
bump app/dōjō/website.

**Open:** title-bar "scribble" (user screenshot next run); website split #346; #282 part 3
(Permissions step); one zz-a11y axe eval timeout that did not recur on rerun (cause unknown).

**Known broken:** Keychain test on rebuilt binaries (#282). sensei MCP checkpoints 404 in this
long session (stale sensei-mcp).
