---
name: Features — sensei
updated: 2026-07-25
---

# Features

The feature index for sensei. Each bullet below becomes its own feature doc
(`docs/features/<name>/` — a question-headed story + tests + decisions +
mockup-ref; the cross-feature modules & layers live in [`../design/`](../design/)).
Who each feature serves is in [`../personas/`](../personas/).

> **Current status of record → [`coverage-map.md`](coverage-map.md).** The `[x]/[~]/[ ]`
> markers below and the per-feature status tables predate the 2026-07-24/25 build wave
> (governance injection, rule packs + ponytail pack, stance + autonomy gate, Tier-3
> billing + seats + console wire, the coverage audit, and the D-EXEC-TEAM design). The
> coverage map is the authoritative built-vs-pending snapshot across mockups, journeys,
> docs, and code; treat it as the source of truth where this index disagrees.

- [Setup](01-setup.md)
- [Configuration](02-config.md)
- [Observatory](03-observatory.md) — observe (behind the scenes) + the daily UX
- [Project](04-project.md) — the per-project window (actions · tasks · intake)
- [Governance](05-governance.md) — the shared team/org control plane (dōjō)
- [Relay](06-relay.md) — remote processing (remote plan · execute · status · HITL)
- [Pricing](07-pricing.md) — the pricing strategy (free · paid org · sponsors)

## 2 · The map — understand the work
- Code + activity graph (files · functions · components · hooks · docs) [x]
- Incremental watcher · scan · reconcile [x]
- Semantic search + context assembly (embeddings) [x]
- Project atlas / visualization — see the graph (structure · calls · communities) [~]
- Solution / multi-repo view — analyze across the repos in one solution [~]

## 3 · Synthetic analysis — the guiding principles (the core)
- Analyzer engine — schedules + runs the analysis passes over each project [x]
- Graph / architecture analysis — cycles · depth · layering [~]
- Duplicates detection [x]
- Communities / clustering [x]
- Document drift — doc ↔ code divergence [x]
- Traceability — requirement / doc ↔ code linkage [~]
- Metrics calculation — FTR · delivery · signals (churn · correction-prone) [x]
- Governance — rules hierarchy (mandatory · scoped · promoted), resolved live [x]

## 4 · Knowledge — what the analysis learns
- Memory + promotion, anchored to doc slots [x]
- Patterns + anti-patterns [x]
- Conventions (house style) [x]

## 5 · Guidance & recommendations — later, built on the analysis
- Insights (mentor voice) + the retrospective [x]
- Recommendations + impact / verdicts [x]
- Front door / intake — read the chunk, recommend a way of working [x]
- Playbook catalog (vibe · mockup-first · spec-driven · gsd · change-flow · debug-flow) [x]
- In-the-moment guidance — push context / rules / patterns / mindsets into the assistant's path [x]
- Playbook → outcome learning [x](dev)

## 6 · Planning & delivery
- Planner — idea/specs → features → phases → value releases (confirm once, then run) ❌
- Baseline / definition-of-done — capability contract (lint · format · coverage · tests · quality · security · design-system) + gates; verify by effect [~]
- Testability / TDD coaching — guide toward tested, verifiable work [~]
- Autonomous multi-day runs — execute planned phases without babysitting "continue"; remote supervise / nudge [~]
- Execution — graph-safe parallelism, contracts-first, mindsets auto-invoked ❌
- Design / mockup subsystem — brief → mockups on the design-system → handoff ❌
- Human surfaces — the app (today · project · doc / plan / mockup / metrics views; view + comment, no editor) [~]

## 7 · Cross-cutting foundations
- Canonical doc spine + scaffold + per-feature dossiers [x]
- Brownfield onboarding — reconstruct the spine from existing code, reconcile drift ❌
- Gateway — route each LLM step to local vs paid models (finish products past the paywall) [x]
- Model inferencing + benchmarking — pick/verify which model handles which task well [~]
- HF model support ❌

## Helpers — the delivery + interaction layer
- Collection of skills, commands, agents, plugin for assisting the llm [x]
- MCP — a way for the llm to access the rich analytical + structural data available for projects [x]
- A way for the user to interact with tools + helpers the way the llm does — to see what works and what does not [~]

## Dōjō — team / org (the bounded plane)
- Shared governance — rules + promotion + federation [x]
- Collective intelligence — shared memories / patterns across the team [~]
- Role consoles — developer · maintainer · lead · admin [~]
- Delivery metrics (DORA) → planner ❌
- Tenants · identities · policies · audit [~]

---

> **How this is organized.** Each bullet → `docs/features/<name>/`: a
> question-headed `feature.md` (what it is · how you use it · behind the scenes ·
> where it fits · who does what · where are the screens) + `tests/` (acceptance,
> product-owner-observable) + `decisions.md` + `mockup-ref.md`. Cross-feature
> **modules & layers** are not per-feature — they live in [`../design/`](../design/).
> Dated build plans are transient (`../plan/`).
