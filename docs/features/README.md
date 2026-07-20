---
name: Features — sensei
updated: 2026-07-20
---

# Features

The feature index for sensei. Each bullet below becomes its own feature doc
(`docs/features/<name>/` — a question-headed story + tests + decisions +
mockup-ref; the cross-feature modules & layers live in [`../design/`](../design/)).
Who each feature serves is in [`../personas/`](../personas/).

**Build order (2026-07-20): analysis first.** The *synthetic analysis*
capabilities (governance, metrics, document drift, graph) are the foundation —
they produce the guiding principles. **Recommendations & guidance come later**,
once the analysis reports give something to guide with. MVP grouping + phases
are decided together, after every bullet is written up.

Status: ✅ built · 🟡 partial · ❌ planned.

## 1 · Capture — the neutral recorder
- Session capture — prompts, tool calls, edits, outcomes, across assistants ✅
- Transcripts + events ingest ✅

## 2 · The map — understand the work
- Code + activity graph (files · functions · components · hooks · docs) ✅
- Incremental watcher · scan · reconcile ✅
- Semantic search + context assembly (embeddings) ✅

## 3 · Synthetic analysis — the guiding principles (the core)
- Graph / architecture analysis — cycles · depth · layering 🟡
- Duplicates detection ✅
- Communities / clustering ✅
- Document drift — doc ↔ code divergence ✅
- Metrics calculation — FTR · delivery · signals (churn · correction-prone) ✅
- Governance — rules hierarchy (mandatory · scoped · promoted), resolved live ✅

## 4 · Knowledge — what the analysis learns
- Memory + promotion, anchored to doc slots ✅
- Patterns + anti-patterns ✅
- Conventions (house style) ✅

## 5 · Guidance & recommendations — later, built on the analysis
- Insights (mentor voice) + the retrospective ✅
- Recommendations + impact / verdicts ✅
- Front door / intake — read the chunk, recommend a way of working ✅
- Playbook catalog (vibe · mockup-first · spec-driven · gsd · change-flow · debug-flow) ✅
- In-the-moment guidance — push context / rules / patterns / mindsets into the assistant's path ✅
- Playbook → outcome learning ✅(dev)

## 6 · Planning & delivery
- Planner — idea/specs → features → phases → value releases (confirm once, then run) ❌
- Baseline / definition-of-done — capability contract (lint · format · coverage · tests · quality · security · design-system) + gates; verify by effect 🟡
- Autonomous multi-day runs — execute planned phases without babysitting "continue"; remote supervise / nudge 🟡
- Execution — graph-safe parallelism, contracts-first, mindsets auto-invoked ❌
- Design / mockup subsystem — brief → mockups on the design-system → handoff ❌
- Human surfaces — the app (today · project · doc / plan / mockup / metrics views; view + comment, no editor) 🟡

## 7 · Cross-cutting foundations
- Canonical doc spine + scaffold + per-feature dossiers ✅
- Brownfield onboarding — reconstruct the spine from existing code, reconcile drift ❌
- Gateway — route each LLM step to local vs paid models (finish products past the paywall) ✅
- HF model support ❌

## Dōjō — team / org (the bounded plane)
- Shared governance — rules + promotion + federation ✅
- Collective intelligence — shared memories / patterns across the team 🟡
- Role consoles — developer · maintainer · lead · admin 🟡
- Delivery metrics (DORA) → planner ❌
- Tenants · identities · policies · audit 🟡

---

> **How this is organized.** Each bullet → `docs/features/<name>/`: a
> question-headed `feature.md` (what it is · how you use it · behind the scenes ·
> where it fits · who does what · where are the screens) + `tests/` (acceptance,
> product-owner-observable) + `decisions.md` + `mockup-ref.md`. Cross-feature
> **modules & layers** are not per-feature — they live in [`../design/`](../design/).
> Dated build plans are transient (`../plan/`).
