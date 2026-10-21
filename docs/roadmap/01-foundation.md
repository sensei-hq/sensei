# Roadmap — Foundation plane (M1–M6)

The floor: provisioning (M1), preferences (M2), capture (M3), the code graph
(M4), inference routing (M5), the data model (M6). Status legend:
[`README.md`](README.md).

> Per-module statuses are filled in Phase 3 from the legacy coverage map and the
> plan gap analysis (both deleted on 2026-10-21; they live in git history).
> Structure is final.

| ID | Module | One-line "done" | Status |
|---|---|---|---|
| M1 | Bootstrap & Provisioning | Fresh install reaches all-green with no manual toolchain work | 🚧 Phase 3 |
| M2 | Setup & Preferences | Every default is adjustable in a searchable surface — incl. icon style (kanji ⇄ solar) and the wider preference surface | 🚧 Phase 3 |
| M3 | Capture & Sessions | Every assistant session is captured with turns, tools and outcome; Claude + OpenCode first, extension-ready | 🚧 Phase 3 |
| M4 | Code Graph & Indexing | A repo scans to a resolved, queryable graph, incrementally | 🚧 Phase 3 |
| M5 | Inference Gateway | Every LLM step routes to a local or paid model under a policy | 🚧 Phase 3 |
| M6 | Storage & Data Model | The schema is the single source of truth and drifts clean | 🚧 Phase 3 |

## Known gaps affecting this plane

| Gap | What | Priority |
|---|---|---|
| ~~A-1~~ | ~~app is kanji-only~~ — the kanji ⇄ icons switch is live (`ui.glyphs`, Settings → General; #133) | — |
| A-2 | the personal-preference surface is narrower than the configuration the app already supports (M2) | P3 |
| C-11 | transcript-scanning review not scoped as a task (M3) | P1 |
| C-13 | Claude "mods" (supersedes plugin + hooks) — install path + migration not verified (M3) | P1 |
| C-14 | multi-assistant expansion: Claude + OpenCode priority; others later; extension seam not specified (M3) | P1 |

Full register: [`06-gaps.md`](06-gaps.md).

## Sequencing

M6 (data model) is the read base; M4's indexer state gates everything downstream
(graph 40.3% resolved, 1.6M/2.4M edges on the live sensei DB — see
`../history/reports/` for the 2026-10-13 placement numbers). C-13/C-14 (M3) are
the facing capture work this quarter.