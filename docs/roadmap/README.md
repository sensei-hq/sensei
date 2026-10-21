# Roadmap — where sensei is, what's next

The **current status** of the product, module by module. The PRD
([`../prd/README.md`](../prd/README.md)) is *what we are building*; this tree is
*is it built, how far, and what's next*. If a module spec and this roadmap
disagree, the roadmap's status is the bug to check first.

## How to read

One file per plane: [`01-foundation.md`](01-foundation.md) · [`02-insight.md`](02-insight.md)
· [`03-delivery.md`](03-delivery.md) · [`04-team.md`](04-team.md) · [`05-platform.md`](05-platform.md).
Each plane file is a table: one row per module — status · what's done · the next
step. The consolidated gap register is [`06-gaps.md`](06-gaps.md); issue-level open
work is [`../backlog.md`](../backlog.md) (GitHub issues).

> **Status of this file:** the per-module status tables are populated in **Phase 3**
> of the reorg ([`../reorg.md`](../reorg.md) §13), folding the old feature
> coverage map and the plan gap analysis (both deleted on 2026-10-21; they live
> in git history). The models below are final now.

## Status model

Six states, matching what the code can actually evidence (from the old coverage
map's legend, kept identical):

| Marker | Meaning |
|---|---|
| ✅ **Built** | shipped + code evidence |
| 🟡 **Partial** | backend or UI present; the other half or wiring missing |
| 🖼️ **Mock-only** | mockup exists, no built surface |
| 📄 **Designed** | doc/design only, no code |
| 💤 **Dormant** | built but behind an off-by-default flag / not merged to main |
| ⛔ **Gap** | intended but neither designed nor built |

Every status must be **evidenced** — a row without "proven by …" is a claim, and
claims unsourced get fixed. The old "these numbers are real" discipline (the
`# pinned:` / `NO TEST` convention, reorg §6) is the evidence style.

## Priority model

| Priority | Meaning | Example |
|---|---|---|
| **P0** | blocks everything else / the loop can't close without it | the FTR loop's closing work, capture indexing |
| **P1** | required for the current target (macOS today; Linux + Windows are required, built in parallel) | P1 Distribution adapters |
| **P2** | should exist for the product to be credible / sellable | the website (marketing site, confirmed 2026-10-13) |
| **P3** | nice-to-have; only after P0–P2 | DORA module, HF-model support |

Priorities are per-module and revisited when a plane file changes. "Shipped"
beats "designed": promotion to ✅ requires evidence (tests, e2e, live data), not
a spec.

## The board — all 29 modules at a glance

| Plane | Modules | Status (Phase 3 populates the cells) |
|---|---|---:|
| Foundation | M1 Bootstrap · M2 Setup/Preferences · M3 Capture · M4 Code Graph · M5 Inference Gateway · M6 Storage/Data Model | [01-foundation.md](01-foundation.md) |
| Insight | M7 Analyzer · M8 Metrics/Health · M9 Memory · M10 Patterns · M11 Traceability · M12 Libraries · M13 Governance | [02-insight.md](02-insight.md) |
| Delivery | M14 MCP/Context · M15 Observatory · M16 Workspace Window · M17 Atlas · M18 Instruments · M19 Insights/Impact · M20 Planner · M21 Relay · M22 Agentic Execution | [03-delivery.md](03-delivery.md) |
| Team | T1 Dōjō Core · T2 Consoles · T3 Workspace Planning · T4 Billing/Seats | [04-team.md](04-team.md) |
| Platform | P1 Distribution · P2 Website · P3 Harness/CI | [05-platform.md](05-platform.md) |

## Sequencing — what gates what

The dependency spine (from `../prd/01-vision.md`'s loop): **capture → graph →
analyze → learn → deliver → measure FTR**. Each plane file lists its own gates;
the cross-plane ones that matter now:

- **M17 (Atlas L0/L1/L2) is a prerequisite for reading architecture metrics at
  any scale** — reorg §9.7; the current layered render is unreadable (G-1).
- **M8 (master-detail + gating eligibility) must land before T3 promises gates** —
  1 of T3's 7 gating targets is buildable today, 2 need scoped reads, 4 are
  blocked on #156 (reorg §9.2).
- **M3 adapter expansion (Claude + OpenCode first)** is the capture plane's
  facing work (decision 10; C-13, C-14).
- **P1 design is finished before implementation distributes across the three
  targets** (decision 7).

## Gaps

The consolidated register: [`06-gaps.md`](06-gaps.md) — every gap is IDed
(`F-*` faults · `P-*` perf · `T-*` test · `D-*` docs/claims · `M-*` metrics ·
`C-*` capture/conventions · `X-*` cross-cutting), carries one line + evidence +
priority, and points at the module it blocks.

## Read next

- [`../prd/README.md`](../prd/README.md) — what we are building (the why).
- [`../backlog.md`](../backlog.md) — the live open-issue index (GitHub).
- [`../history/README.md`](../history/README.md) — the small archive of
  superseded documents that live docs still cite.
- [09 — Dōjō expansion](09-dojo-expansion.md): workspaces, teams and roles authored in the dōjō, dōjō ⇄ sensei sync, integrations, release pipelines and DORA (decisions 2026-10-21).
