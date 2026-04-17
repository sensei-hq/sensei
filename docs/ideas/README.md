---
name: Ideas
description: Ideation artifacts — problem statements, concepts, and early-stage thinking for the sensei system
---

# Ideas

## Core concepts

| # | Concept | Description |
|---|---------|-------------|
| 1 | **Workflow** | Phased development workflow for the AI assistant — structure, commands, configuration, quality gates |
| 2 | **Logging & Qualitative Analysis** | Track interactions, sessions, and outcomes to provide qualitative guidance and coaching feedback |
| 3 | **Metrics & Quantitative Analysis** | FTR scoring, turn counts, rework rates, trend visualization for measurable improvement |
| 4 | **Assistive Tooling** | Precision assistance — codebase intelligence, library knowledge, context delivery, pattern enforcement |
| 5 | **Knowledge Integrity** | Ensure the AI's information sources are accurate — doc traceability, drift detection, doc linting |
| 6 | **Platform & Adoption** | Make sensei usable beyond Claude Code — multi-coordinator support, plugin packaging, skill transformation |

---

## 1. Workflow

Phased development workflow that reduces rework cycles, preserves context across sessions, and adapts to different project needs.

| # | Document | Status |
|---|----------|--------|
| 01 | [Workflow System](./01-workflow-system.md) | Complete |
| 02 | [Commands](./02-commands.md) | Complete |
| 03 | [Configuration](./03-configuration.md) | Complete |
| 04 | [Cross-Cutting Concerns](./04-cross-cutting.md) | Complete |
| 05 | [Decisions](./05-decisions.md) | Complete |
| 06 | [Docs Disposition](./06-docs-disposition.md) | Complete |

---

## 2. Logging & Qualitative Analysis

Track interactions, sessions, and outcomes. Provide qualitative guidance — coaching feedback, session recovery, handoff documents.

| # | Document | Status |
|---|----------|--------|
| 07 | [Metrics & Analytics](./07-metrics-analytics.md) — coaching layer, interaction tracking | Idea |
| 11 | [Session Continuity](./11-session-continuity.md) — recovery, interrupt detection, handoff | Idea |

---

## 3. Metrics & Quantitative Analysis

Measure development quality with hard numbers. FTR scoring, turn counts, rework rates, trend visualization.

| # | Document | Status |
|---|----------|--------|
| 07 | [Metrics & Analytics](./07-metrics-analytics.md) — FTR, telemetry, benchmarking, dashboards | Idea |
| 10 | [Visualization & Dashboard](./10-visualization.md) — quality trends, session analytics | Idea |

---

## 4. Assistive Tooling

Precision assistance — understand the codebase deeply, serve the right context, know the libraries, enforce the patterns.

| # | Document | Status |
|---|----------|--------|
| 08 | [Codebase Intelligence](./08-codebase-intelligence.md) — indexing, call graph, graph analysis, pattern detection | Idea |
| 09 | [Library Intelligence](./09-library-intelligence.md) — doc indexing, registry, skill generation | Idea |
| 14 | [Context Delivery](./14-context-delivery.md) — resolution levels, token budgeting, ranking | Idea |
| 15 | [Pattern Store](./15-pattern-store.md) — detect, surface, enforce, grow lifecycle | Idea |
| 17 | [Pattern Knowledge](./17-pattern-knowledge.md) — library patterns, industry patterns, architectural options | Idea |
| 18 | [Testability & TDD](./18-testability-tdd.md) — composable functions, test-first with human approval, testability scoring | Idea |

---

## 5. Knowledge Integrity

Ensure the AI's information sources are accurate and current. Stale docs mislead; drift erodes trust.

| # | Document | Status |
|---|----------|--------|
| 13 | [Documentation Traceability](./13-doc-traceability.md) — doc-to-code links, drift detection, linting | Idea |

---

## 6. Platform & Adoption

Make sensei usable beyond Claude Code. Multi-repo, multi-coordinator, multi-team.

| # | Document | Status |
|---|----------|--------|
| 12 | [Multi-Coordinator Support](./12-multi-coordinator.md) — adapter pattern for Cursor, Copilot, etc. | Idea |
| 16 | [Workspace & System Intelligence](./16-workspace-system-intelligence.md) — multi-repo, conformance, health | Idea |
