---
name: Workflow System — Cross-Cutting Concerns
description: Quality, guardrails, tool hierarchy, context management, phase gates, human training, and metrics
date: 2026-04-17
parent: 01-workflow-system.md
---

# Cross-Cutting Concerns

These apply across all phases and shape how the AI behaves throughout the workflow.

---

## Code quality

- `/sensei:review` auto-triggers after each feature completion in `/sensei:build`
- Also available on demand at any phase
- Configurable strictness: "strict" (block on issues), "advisory" (warn), "minimal" (on request)
- Detects: duplication, pattern drift, stale docs, missing tests, SOLID violations

---

## Guardrails as a living document

- Project guardrails file (e.g., `.sensei/guardrails.md`) captures patterns, constraints, and quality rules
- Guardrails **grow from feedback**: when the user corrects the AI ("you didn't use the adapter pattern"), the AI asks clarifying questions and adds the pattern to guardrails
- Next session, guardrails are loaded automatically — the correction persists without the user repeating it
- This is different from memory (which stores facts about the user/project) — guardrails store **enforceable rules about how to build**

---

## Tool preference hierarchy

- When a graph/MCP tool can answer a question, prefer it over grep/sed
- SessionStart hook reminds AI of available tools for this project
- `/sensei:tools` command reloads awareness mid-session after compaction
- Pre-action check: before falling back to built-in tools, check if a project-specific tool exists

---

## Context management

- Each phase command loads only the artifacts relevant to that phase
- Flush context from completed phases (keep the doc, not the working memory)
- The graph DB provides focused lookups — use it instead of reading entire files
- PreCompact hook auto-fires lightweight refocus to preserve critical context across compaction
- `/sensei:refocus` available manually when the human notices drift

---

## Phase gates (soft, not hard)

- AI nudges when details are insufficient: "I don't have enough to build on — should we `/sensei:experiment` first?"
- AI suggests phase transitions: "this feels deep enough for ideation — ready for `/sensei:analyze`?"
- Never blocks outright — the human always has the final say
- In auto-mode, the nudge is especially important since auto-mode's default is to skip deliberation

---

## Human training

- `/sensei:idea` asks structured questions to force clarity before work begins
- `/sensei:experiment` legitimizes "I don't know yet" as a valid phase with its own rules
- `/sensei:brainstorm` is a decision-making tool, not just open discussion
- Phase transitions prompt reflection: "what did we learn? what changed?"

---

## Metrics and improvement tracking

- Track interactions: user messages, AI responses, outcomes
- Use `/sensei:analyze` on interaction data to derive FTR (first-time-right), turn count, rework rate
- Requires instrumentation of the conversation itself, not just code outcomes
