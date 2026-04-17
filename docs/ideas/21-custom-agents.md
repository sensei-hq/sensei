---
name: Custom Agents
description: Specialized autonomous agents that go beyond commands/skills — deeper integration with the sensei ecosystem for complex multi-step workflows
date: 2026-04-17
status: idea
related: 01-workflow-system.md, 18-testability-tdd.md, 15-pattern-store.md
---

# Custom Agents

## Problem

Commands are instructions the AI reads and follows. Skills auto-trigger on patterns. Both are single-turn — they guide one interaction. But complex workflows like code review, build cycles, and codebase analysis need **multi-step autonomous execution** with their own system prompt, tool restrictions, and specialized behavior. An agent can run a full review pass without user intervention, report findings, and log events — something a command can't do reliably.

## Why agents over commands

| Aspect | Command | Skill | Agent |
|--------|---------|-------|-------|
| Trigger | User types `/sensei:X` | Auto on pattern match | User invokes or spawned by another agent |
| Execution | AI follows instructions once | AI follows instructions once | Runs autonomously with own system prompt |
| Tool access | All tools | All tools | Restricted to specific tools |
| Context | Shares main conversation | Shares main conversation | Isolated context (can be focused) |
| Multi-step | Relies on AI memory within turn | Single turn | Can loop, retry, branch |
| State | Manual (AI must remember) | Manual | Managed by agent framework |

## Potential sensei agents

| Agent | What it does | Why it's better than a command |
|-------|-------------|-------------------------------|
| **review-agent** | Multi-pass code review: patterns → duplicates → test coverage → doc drift | Command does one pass. Agent can iterate, cross-reference findings, prioritize. |
| **build-agent** | Full build cycle: locate → decompose → test → implement → review | Command relies on AI holding all steps in context. Agent manages state between steps. |
| **analyze-agent** | Deep codebase analysis: scan all modules, identify patterns, produce comprehensive report | Command would overload main context. Agent works in isolation, reports summary. |
| **index-agent** | Run indexer, verify graph quality, flag issues | Can retry failed files, handle timeouts, report progress. |
| **benchmark-agent** | Run A/B benchmarks, collect metrics, generate comparison report | Long-running, needs isolation from main conversation. |
| **onboard-agent** | New project setup: detect stack, index, create rules, configure | Multi-step setup that adapts based on what it finds. |

## How agents integrate with sensei

Agents would use the same MCP tools as commands, plus:
- Read/write `.sensei/state.yaml` for workflow state
- Call `log_event()` for event capture
- Read `.sensei/rules.md` for project rules
- Access the graph for code intelligence

The daemon doesn't need to know about agents — they're plugin-level (marketplace). The MCP tools are the interface.

## Agent structure (Claude Code plugin format)

```
marketplace/agents/
  review-agent.md       ← system prompt, tool restrictions, behavior
  build-agent.md
  analyze-agent.md
```

Each agent file has frontmatter defining:
- `name` — agent identifier
- `description` — when to use (for auto-triggering or user invocation)
- `tools` — restricted tool list (e.g., review-agent only gets read tools, not write)
- `model` — can use a different model (e.g., haiku for fast passes, opus for deep analysis)
- `color` — visual indicator in the UI

## Open questions

| # | Question |
|---|----------|
| 1 | Should agents be invocable via `/sensei:agent review` or have their own commands `/sensei:deep-review`? |
| 2 | Can agents spawn sub-agents? (e.g., build-agent spawns review-agent after implementation) |
| 3 | How do agents report back to the main conversation? Summary only, or full findings? |
| 4 | Should agents run in worktrees for isolation, or in the same working directory? |
| 5 | How do we test agents? The skill-creator eval framework or something more specialized? |
| 6 | Which agent to build first? review-agent seems highest value — catches quality issues autonomously. |
