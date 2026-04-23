---
name: Ideas
description: Conceptual explorations — problem statements, capabilities, and early-stage thinking for the sensei system
---

# Ideas

What the system could be. For how users experience it, see [journeys](../journeys/).

---

## Workflow & Session Management

How AI assistants work with sensei: phases, commands, context, continuity.

| # | Document | Status |
|---|----------|--------|
| 01 | [Workflow System](./01-workflow-system.md) — phased development: brainstorm → build → validate | Complete |
| 02 | [Commands](./02-commands.md) — slash commands for each phase + utilities | Complete |
| 04 | [Cross-Cutting Concerns](./04-cross-cutting.md) — guardrails, tool preferences, compact handling | Complete |
| 11 | [Session Continuity](./11-session-continuity.md) — recovery, interrupt detection, handoff | Idea |
| 23 | [Personas & Mindsets](./23-personas-mindsets.md) — process mindsets + project personas | Idea |
| 27 | [Developer Preferences](./27-developer-preferences.md) — learn personal style from corrections + codebase scan | Idea |

---

## Codebase Intelligence

Understand code deeply: index, graph, patterns, duplicates, complexity.

| # | Document | Status |
|---|----------|--------|
| 08 | [Codebase Intelligence](./08-codebase-intelligence.md) — indexing, call graph, community detection | Idea |
| 15 | [Pattern Store](./15-pattern-store.md) — detect, surface, enforce (suggested → gap → rule) | Idea |
| 17 | [Pattern Knowledge](./17-pattern-knowledge.md) — library patterns, industry patterns, architectural options | Idea |
| 14 | [Context Delivery](./14-context-delivery.md) — resolution levels (L0-L3), token budgeting, ranking | Idea |
| 18 | [Testability & TDD](./18-testability-tdd.md) — composable functions, test-first, testability scoring | Idea |
| 22 | [Adapter IR](./22-adapter-ir.md) — common intermediate representation for language parsing | Complete |

---

## Library & Service Intelligence

Index documentation, track usage, detect drift, wrap libraries without their own MCP.

| # | Document | Status |
|---|----------|--------|
| 09 | [Library Intelligence](./09-library-intelligence.md) — doc indexing, library wrapping, skill generation | Idea |
| 13 | [Documentation Traceability](./13-doc-traceability.md) — doc-to-code links, drift detection, linting | Idea |

---

## Metrics, Analytics & Coaching

Measure session effectiveness. FTR, corrections, rework, coaching recommendations.

| # | Document | Status |
|---|----------|--------|
| 07 | [Metrics & Analytics](./07-metrics-analytics.md) — FTR scoring, coaching, session analytics, correction clustering | Active |
| 10 | [Visualization & Dashboard](./10-visualization.md) — quality trends, session analytics, graph visualization | Idea |
| 29 | [Collective Intelligence Network](./29-telemetry.md) — share derived insights across all users to improve skills, agents, and defaults for everyone | Idea |

---

## Insights, Playground & Continuous Improvement

Tool exploration, session replay, MOE reasoning, change-impact tracking.

| # | Document | Status |
|---|----------|--------|
| 25 | [Playground & Insights Engine](./25-playground-and-insights.md) — MCP playground, session replay, usage analytics, MOE consensus, change-impact tracking | Idea |
| 20 | [Local Inference](./20-local-inference.md) — Ollama, Gemma4, embeddings, classification, reasoning panel | Idea |
| 19 | [Benchmarking & Credibility](./19-benchmarking-credibility.md) — effectiveness benchmarks, reproducible comparison | Idea |
| 28 | [Inference Gateway](./28-inference-gateway.md) — Rust LLM router: Ollama + external providers, fallback chains, circuit breaker, MOE consensus, budget management. Port of Strategos gateway. | Idea |

---

## Desktop & Observatory

The visual layer: setup wizard, daily view, project pages, navigation.

| # | Document | Status |
|---|----------|--------|
| 24 | [Desktop Observatory](./24-desktop-observatory.md) — session observatory, setup wizard, scope model | Idea |
| 24a | [Observatory Data Audit](./24a-observatory-data-audit.md) — data requirements for observatory views | Idea |
| 24b | [Capability Registry](./24b-capability-registry.md) — skills, plugins, MCP registration | Idea |
| 26 | [Bootstrap & Dependencies](./26-bootstrap-and-dependencies.md) — startup screen, Homebrew, PostgreSQL, Ollama, models | Idea |

---

## Platform & Extensibility

Multi-coordinator, plugins, agents, custom skills.

| # | Document | Status |
|---|----------|--------|
| 12 | [Multi-Coordinator Support](./12-multi-coordinator.md) — adapter pattern for Cursor, Copilot, Codex, Aider | Idea |
| 16 | [Workspace & System Intelligence](./16-workspace-system-intelligence.md) — multi-repo, conformance, health | Idea |
| 21 | [Custom Agents](./21-custom-agents.md) — autonomous multi-step agents for review, build, analysis | Idea |

---

## Configuration & Housekeeping

| # | Document | Status |
|---|----------|--------|
| 03 | [Configuration](./03-configuration.md) — config schema, project vs user scope, defaults | Complete |
| 05 | [Decisions](./05-decisions.md) — architectural decision records | Complete |
| 06 | [Docs Disposition](./06-docs-disposition.md) — what to keep, archive, retire | Complete |
