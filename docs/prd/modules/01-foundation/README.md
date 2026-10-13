# Foundation plane — the floor everything stands on

> Foundation = M1–M6. Without this plane nothing is captured, nothing is
> indexed, and nothing can be reasoned about. Status:
> [`../../../roadmap/01-foundation.md`](../../../roadmap/01-foundation.md).

| ID | Module | One-line "done" | Spec |
|---|---|---|---|
| M1 | Bootstrap & Provisioning | Fresh install reaches all-green with no manual toolchain work | ✅ [`bootstrap-provisioning/`](bootstrap-provisioning/README.md) · 2026-10-13 |
| M2 | Setup & Preferences | Every default is adjustable in a searchable surface — incl. the icon-style (kanji ⇄ solar) preference | ✅ [`setup-preferences/`](setup-preferences/README.md) · 2026-10-13 |
| M3 | Capture & Sessions | Every assistant session is captured with turns, tools and outcome — Claude, OpenCode first, others behind a shared adapter seam | ✅ [`capture-sessions/`](capture-sessions/README.md) · 2026-10-13 |
| M4 | Code Graph & Indexing | A repo scans to a resolved, queryable graph, incrementally | ✅ [`code-graph/`](code-graph/README.md) · 2026-10-13 |
| M5 | Inference Gateway | Every LLM step routes to a local or paid model under a policy | ✅ [`inference-gateway/`](inference-gateway/README.md) · 2026-10-13 |
| M6 | Storage & Data Model | The schema is the single source of truth and drifts clean | ✅ [`storage-data-model/`](storage-data-model/README.md) · 2026-10-13 |

## What this plane owns

The capture→index base of the loop: provisioning and health (M1), the
preference/config surface (M2), session capture across assistant adapters (M3),
the code graph and its incremental indexer (M4), LLM routing (M5), and the
schema everything else reads (M6).

## Reading order

M6 (the data model) → M1 → M4 → M3 → M2 → M5. Start with M6 if you need to know
what a row means; start with M1 if you are wiring a fresh machine.

## Customer

Every other plane. Foundation defects are silent across Insight and Delivery —
this is the plane where "no error" must never be trusted as "no work".

## Legacy sources

Folded from `features/01-setup.md`, `features/02-config.md`,
`features/front-door/`, `spec/screen/{bootstrap-*,first-run-*,preferences,
settings-inference}`, `spec/pipeline/{capture,code-graph,inferencing,
bootstrap-resolution}`, `architecture/{cli,daemon,data,app,frontend-svelte-guidelines}`.