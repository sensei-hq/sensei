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
| D11 | `/sensei:brainstorm` is the open conversation; phase commands set intent | Brainstorm is the primary creative command — one conversation can produce artifacts at multiple depth levels. The AI routes content to the right folder (ideas/, analysis/, blueprints/) based on depth. Phase commands (`/sensei:analyze`, `/sensei:blueprint`) are intent signals that set focus and constraints but don't lock output to a single folder. Observed: our ideation session naturally produced idea, analysis, and design-depth content — forcing it all into `docs/ideas/` was wrong. |
| D12 | Content finds its natural level, not the phase folder | A skill/command should place design-depth content in `docs/blueprints/`, analysis-depth in `docs/analysis/`, regardless of which command initiated the work. Frontmatter `origin:` traces lineage back to the parent idea. |
| D13 | Mark stale MCP references for removal during implementation | 10 skills reference MCP tools from the old JS server that may not match Rust contracts. Don't fix now — mark them and address during build phase. |
| D14 | Workflow state file (`.sensei/state.yaml`) as single source of truth | Commands update it, hooks read it, `/sensei:status` displays it. Solves "where am I" problem. Also the testable artifact — verify state after each command. |
| D15 | `/sensei:status` command for orientation | The "where am I" query. Reads state file + project structure. Callable by AI internally when lost, not just user-facing. Pre-compact hook reminds AI to call it after compaction. |
| D16 | GitHub issues as backlog for GitHub repos | Markdown backlogs go stale. GitHub issues are persistent, queryable, and manageable independently. Flow: feature doc → GitHub issue → AI picks issue → works → closes with commit. Fall back to `docs/backlog.md` for non-GitHub repos. |
| D17 | Roadmap decomposed into GitHub issues with wave milestones | The entire roadmap becomes issues. One at a time, verified. Labels: concept, depth, wave, priority, type. |
