---
name: Workflow System — Decisions Log
description: Design decisions made during ideation with rationale for traceability
date: 2026-04-17
parent: 01-workflow-system.md
---

# Decisions Log

Decisions made during ideation, preserved for traceability. Each decision includes the rationale so future sessions can understand *why*, not just *what*.

| # | Decision | Rationale |
|---|----------|-----------|
| D1 | Archive existing `docs/superpowers/plans` and `docs/superpowers/specs` | Stale artifacts from prior architecture. Move to `docs/_archive/superpowers/`. |
| D2 | Commands are the product; workflows are suggested sequences | Users can invoke any command at any time. Recipes are onboarding templates, not enforced pipelines. Config is global + project-level override. |
| D3 | Soft phase gates — nudge, don't block | AI politely informs when details are insufficient and suggests `/sensei:experiment`. Never refuses to proceed. |
| D4 | Track interaction metrics for improvement measurement | FTR, turn count, rework rate derived from conversation instrumentation. |
| D5 | Reorganize existing `docs/design/` and `docs/features/` | Remove stale references, note architectural evolution (SaaS → local-first → Rust). Historical context is valuable; stale docs pretending to be current are not. |
| D6 | Replace existing skills with phase commands | Skills like working-smarter, context-efficiency, session-management become behaviors embedded in phase commands, not standalone skills. |
| D7 | Auto-trigger `/sensei:review` after each feature in `/sensei:build` | Also available on demand. |
| D8 | Auto-fire lightweight refocus via PreCompact hook | Complements manual `/sensei:refocus`. Both mechanisms: autopilot + steering wheel. |
| D9 | Guardrails are a living document at project level | Grow organically from feedback. AI asks clarifying questions and incorporates corrections into guardrails file. Loaded automatically next session. |
| D10 | Namespace all commands under `sensei:` | Eliminates current and future collisions with Claude Code built-ins (`/plan`, `/review`) and other plugins. Consistency over convenience. |
