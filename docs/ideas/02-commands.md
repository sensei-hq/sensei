---
name: Workflow System — Commands
description: Complete command surface for the sensei workflow system — phase, cross-cutting, refocus, and utility commands
date: 2026-04-17
parent: 01-workflow-system.md
---

# Commands

All commands are namespaced under `sensei:` to avoid collisions with Claude Code built-in commands (`/plan`, `/review`, etc.) and other plugins.

---

## Phase commands

Each command sets the active phase, loads relevant prior artifacts, and constrains behavior.

| Command | Phase | Behavior |
|---------|-------|----------|
| `/sensei:idea` | 01 Ideate | Structured brainstorm. AI asks clarifying questions, documents problem/goals/constraints. Output: idea doc. No code. |
| `/sensei:analyze` | 02 Analyze | Reads idea doc + scans codebase. Produces feasibility assessment with 2-3 options and tradeoffs. No code. Subsumes the former `/sensei:product` and `/sensei:feature` commands (reverse-engineering and feature deep-dives are modes of analysis). |
| `/sensei:blueprint` | 03 Blueprint | Reads chosen approach. Produces architecture: components, interfaces, data flow, integration points. No code. |
| `/sensei:experiment` | 04 Experiment | Creates branch. Build throwaway code to test assumptions. Produces findings doc. Code is discardable but structured for potential incorporation. |
| `/sensei:plan` | 05 Plan | Reads blueprint. Decomposes into ordered features with acceptance criteria and test scenarios. |
| `/sensei:build` | 06 Build | TDD cycle. Reads plan, picks next task, writes tests first, implements. One feature at a time. Auto-triggers `/sensei:review` after each feature. |
| `/sensei:validate` | 07 Validate | Runs E2E tests, checks integration, detects doc drift, produces quality report. |

---

## Cross-cutting commands

Available at any phase. Do not advance the phase — they operate within it.

| Command | Purpose | Behavior |
|---------|---------|----------|
| `/sensei:brainstorm` | Open creative conversation | The primary creative command. One conversation can produce artifacts at multiple depth levels — ideas, analysis, design. The AI routes content to the appropriate folder (`docs/ideas/`, `docs/analysis/`, `docs/blueprints/`) based on depth, using frontmatter `origin:` to trace lineage. Phase commands set intent within a brainstorm; brainstorm is the container. (Decision D11) |
| `/sensei:review` | Quality check | Duplication, pattern adherence, test coverage, doc accuracy at current stage. Auto-triggers after each `/sensei:build` feature. Also available on demand. Subsumes the former `/sensei:audit` command (OWASP/NFR auditing is a form of quality review). |

---

## Refocus commands

These counteract context decay in long conversations. After compaction, hooks and early guidelines disappear from the AI's working memory. These commands explicitly reload constraints and re-anchor the session.

A lightweight refocus also auto-fires via PreCompact hook. The manual commands complement the hook — autopilot + steering wheel.

| Command | Purpose | Behavior |
|---------|---------|----------|
| `/sensei:guardrails` | Reload constraints | Re-reads project guardrails: workflow config, quality policy, tool preferences, active phase rules. Outputs a compact summary of what's in force. Acts as a mid-session reset. |
| `/sensei:patterns` | Reload patterns | Re-reads `PATTERNS.md` and active design patterns for the current task. Ensures the AI builds on existing foundations rather than reinventing. Subsumes the former `/sensei:pattern-use` command. |
| `/sensei:refocus` | Re-anchor on current task | Re-reads the active phase document + plan + current task acceptance criteria. Flushes tangential context. Outputs: "here's where we are, here's what's left." Subsumes the former `/sensei:backlog` command (surfacing open work is part of re-anchoring). |
| `/sensei:tools` | Reload tool awareness | Re-reads available MCP tools, their purposes, and the preference hierarchy. Counteracts the AI's tendency to forget external tools exist after compaction. |

---

## Utility commands

These serve purposes orthogonal to the workflow phases. Retained from the current plugin.

| Command | Purpose | Notes |
|---------|---------|-------|
| `/sensei:session` | Resume session — loads interrupted work, open decisions, orients the AI | Entry point for every new conversation. Orthogonal to phases. |
| `/sensei:checkpoint` | Snapshot current progress for interruption recovery | Used mid-session at key decision points. Works within any phase. |
| `/sensei:commit` | Zero-errors check then git commit | Enforces quality gate before committing. Used during `/sensei:build`. |
| `/sensei:mockup` | Start a UI mockup — framework-native, commits first | Distinct purpose: UI exploration. Used during `/sensei:experiment` or `/sensei:build`. |
| `/sensei:pattern-extract` | Extract a reusable pattern from code into PATTERNS.md | Distinct purpose: pattern discovery. Invoked after noticing recurring structure. |
| `/sensei:get-api-docs` | Fetch third-party library docs before writing code | Distinct purpose: library knowledge. Used during any coding phase. |
| `/sensei:help` | Show all available sensei commands and skills | Reflects the full command surface. |
| `/sensei:enable` | Enable an opt-in skill for this project | Retained; may evolve into the configuration model. |
| `/sensei:disable` | Disable an opt-in skill for this project | Retained; may evolve into the configuration model. |

---

## Retired commands

These are absorbed into the new workflow commands and will be removed.

| Old command | Absorbed into | Reason |
|-------------|---------------|--------|
| `/sensei:product` | `/sensei:analyze` | Product reverse-engineering is a mode of analysis. |
| `/sensei:feature` | `/sensei:analyze` | Feature deep-dive is a mode of analysis. |
| `/sensei:audit` | `/sensei:review` | OWASP/NFR/quality auditing is a form of quality review. |
| `/sensei:backlog` | `/sensei:refocus` | Surfacing open work is part of re-anchoring on current state. |
| `/sensei:pattern-use` | `/sensei:patterns` | Loading and applying patterns is the refocus command's job. |

---

## Full command surface (22 commands)

| # | Command | Category |
|---|---------|----------|
| 1 | `/sensei:idea` | Phase |
| 2 | `/sensei:analyze` | Phase |
| 3 | `/sensei:blueprint` | Phase |
| 4 | `/sensei:experiment` | Phase |
| 5 | `/sensei:plan` | Phase |
| 6 | `/sensei:build` | Phase |
| 7 | `/sensei:validate` | Phase |
| 8 | `/sensei:brainstorm` | Cross-cutting |
| 9 | `/sensei:review` | Cross-cutting |
| 10 | `/sensei:guardrails` | Refocus |
| 11 | `/sensei:patterns` | Refocus |
| 12 | `/sensei:refocus` | Refocus |
| 13 | `/sensei:tools` | Refocus |
| 14 | `/sensei:session` | Utility |
| 15 | `/sensei:checkpoint` | Utility |
| 16 | `/sensei:commit` | Utility |
| 17 | `/sensei:mockup` | Utility |
| 18 | `/sensei:pattern-extract` | Utility |
| 19 | `/sensei:get-api-docs` | Utility |
| 20 | `/sensei:help` | Utility |
| 21 | `/sensei:enable` | Utility |
| 22 | `/sensei:disable` | Utility |
