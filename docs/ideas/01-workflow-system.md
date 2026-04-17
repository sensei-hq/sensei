---
name: Workflow System
description: A phased development workflow that reduces rework cycles, preserves context across sessions, and adapts to different project needs
date: 2026-04-17
status: ideation-complete
origin: conversation — observed failure modes in AI-assisted development over ~6 weeks of building sensei
---

# Workflow System

## Problem

AI-assisted development with Claude Code produces high-quality output when the task is well-defined and small. But quality degrades when:

1. **Requirements are unclear** — the human hasn't fully thought through what they want, the AI proceeds anyway, producing throwaway work
2. **Context is lost between sessions** — design decisions, patterns, constraints evaporate; each session starts cold
3. **No phase discipline** — ideation, design, and implementation blur together; auto-mode amplifies this by skipping deliberation
4. **Cross-cutting quality erodes** — duplication, pattern drift, stale docs accumulate because reviews happen reactively (when things break) rather than systematically
5. **Tools are forgotten** — MCP servers, skills, and commands exist but the AI defaults to built-in primitives; external tools require trust that isn't reinforced

### Observed failure modes (sensei project)

| Failure | Root cause | Cost |
|---------|-----------|------|
| Built features in isolation, not extending existing patterns | No design phase loaded existing patterns first | Rework to integrate |
| Skipped TDD even when skill was available | No enforcement mechanism; auto-mode jumps to code | Bugs found late, test coverage gaps |
| MCP tools ignored in favor of grep/sed | AI defaults to built-in tools; external tools forgotten across sessions | Slower, less accurate code navigation |
| Documents drifted from implementation | No review phase to detect drift | Stale docs mislead future sessions |
| Large tasks produced sloppy results | No decomposition into focused features | Context overload, lost coherence |
| Experiments bled into production code | No separation between exploration and committed work | Cleanup overhead |

---

## Core Principles

### 1. Documents are the memory

Every phase produces an artifact. The AI reads documents to recover context — it doesn't need to "remember." Document quality determines the quality of the next phase. Frontmatter carries metadata (status, phase, dependencies, decisions).

### 2. Intent constrains behavior

A phase command (e.g., `/sensei:blueprint`) sets a contract: what kind of output is expected, where it goes, what prior docs to load, and what depth is appropriate. The command is a **mode**, not just an action.

### 3. The AI is a process guardian, not just an executor

The AI should:
- Suggest when a phase is deep enough and the next phase should begin
- Flag when clarity is missing and ask for it before proceeding
- Refuse to write code when in a brainstorm/design phase
- Load and reference prior phase artifacts rather than starting from scratch

### 4. Workflows are recipes, not rules

Different projects, people, and stages need different levels of ceremony. A solo experiment needs less than a production feature. The system provides configurable workflow recipes; the user picks one or customizes.

### 5. Focused context beats large context

The biggest efficiency gain isn't writing more code faster — it's loading **only what's relevant** for the current task. The graph, the docs, the phase artifacts exist to narrow attention. Flush unused context; load what the task needs.

### 6. Quality is a cross-cutting concern, not a phase

Code review, duplication detection, pattern adherence, and performance awareness happen **within** phases, not as a separate step. Some users want strict quality gates; others want speed. The workflow adapts.

---

## Phases

| ## | Phase | Intent | Input | Output artifact | Gate |
|----|-------|--------|-------|-----------------|------|
| 01 | **Ideate** | Explore a concept, document the problem space | Vague idea, problem statement | `docs/ideas/<name>.md` | Human reviews: "is this worth pursuing?" |
| 02 | **Analyze** | Assess feasibility against existing code | Idea doc + codebase | `docs/analysis/<name>.md` — feasibility, existing patterns, risks, 2-3 approaches with tradeoffs | Human picks an approach |
| 03 | **Blueprint** | High-level solution architecture | Chosen approach from analysis | `docs/blueprints/<name>.md` — components, interfaces, data flow, integration points | Human approves or iterates |
| 04 | **Experiment** | Try options, produce findings (not production code) | Blueprint OR raw idea | Branch-based, findings doc in `docs/experiments/<name>.md` — what worked, what didn't, recommendation | Human decides: incorporate or discard |
| 05 | **Plan** | Decompose into implementable features | Approved blueprint | `docs/plans/<name>.md` — ordered tasks with acceptance criteria, test scenarios per task | Human confirms scope |
| 06 | **Build** | Implement with TDD discipline | Plan + specific task | Code + tests. One feature at a time. | Tests pass, patterns followed |
| 07 | **Validate** | End-to-end verification | Completed build | Test results, integration check, doc-drift review | Human accepts |

### Roadmap

```
Phase 01 ─── Phase 02 ─── Phase 03 ─── Phase 05 ─── Phase 06 ─── Phase 07
 Ideate       Analyze      Blueprint      Plan         Build       Validate
   │              │            │                          ▲
   │              │            ▼                          │
   │              │       Phase 04 ───────────────────────┘
   │              │       Experiment
   │              │        (loops back to Build if viable,
   │              │         or back to Analyze if rethink needed)
   │              │
   ▼              ▼
  ┌──────────────────────────────────────────────────────────────────────────────┐
  │  Cross-cutting: /sensei:brainstorm  /sensei:review                         │
  │  Refocus:       /sensei:rules  /sensei:patterns  /sensei:refocus      │
  │                 /sensei:tools                                               │
  │  Available at any phase. Do not advance the phase.                         │
  └──────────────────────────────────────────────────────────────────────────────┘
```

### Phase flow is not always linear

Workflows define which phases are used and the entry point. Users can jump to any phase at any time — the command sets the active phase regardless of prior sequence.

- **Greenfield**: 01 → 02 → 03 → 05 → 06 → 07
- **Brownfield**: 02 (gap analysis) → 05 → 06 → 07
- **Enhancement**: 02 (existing code) → 03 (extends foundations) → 05 → 06 → 07
- **Experiment**: 01 → 04 → (if viable) 02 → 03 → ...
- **Quick fix**: 02 → 06 → 07 (minimal ceremony for small, well-understood changes)

---

## Related documents

| File | Contents |
|------|----------|
| [02-commands.md](02-commands.md) | Full command surface — phase, cross-cutting, refocus, utility commands. Retired commands and migration notes. |
| [03-configuration.md](03-configuration.md) | Workflow recipes, two-layer config model, per-command overrides. |
| [04-cross-cutting.md](04-cross-cutting.md) | Quality, guardrails, tool hierarchy, context management, phase gates, human training, metrics. |
| [05-decisions.md](05-decisions.md) | D1–D10 decisions log from ideation. |
| [06-docs-disposition.md](06-docs-disposition.md) | Plan for reorganizing existing `docs/` folders. |

---

## Next Steps

1. **This document** is the ideation artifact — status: complete.
2. `/sensei:analyze` — assess existing sensei plugin code, skills, MCP tools, and docs against this ideation. What can be reused? What needs rebuilding? What are the gaps?
3. `/sensei:blueprint` — architect the workflow engine: how commands, phases, docs, config, guardrails, and the graph interact.
4. `/sensei:plan` — decompose into implementable features with acceptance criteria.
5. `/sensei:build` — implement one command at a time, TDD, with `/sensei:review` after each.
