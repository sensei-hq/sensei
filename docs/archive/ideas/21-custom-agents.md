---
name: Custom Agents
description: Agents wrap mindsets (why+what→+how) in marketplace/agents/; generic persona agent loads any persona for task execution
date: 2026-04-17
updated: 2026-04-20
status: accepted
related: 01-workflow-system.md, 18-testability-tdd.md, 15-pattern-store.md
---

# Custom Agents

## Problem

Mindsets tell the AI **what** to think about and **why** — but not **how** to execute autonomously. Commands are single-turn instructions the user triggers. For deeper, multi-step verification and analysis, we need agents that can run autonomously in isolated context with their own tool restrictions, procedures, and report format.

## Core idea: wrap mindsets in agents

Mindsets already define the right questions. Agents add the execution layer:

| Layer | Contains | Execution |
|-------|----------|-----------|
| **Mindset** (what + why) | Questions to ask, principles to follow | Passive — loaded at session start |
| **Agent** (what + why + how) | Same questions PLUS procedures, tools, report format | Active — runs autonomously in isolated context |

The agent **includes** the full mindset content. It doesn't replace it. The mindset stays in `marketplace/mindsets/`, the agent lives in `marketplace/agents/`.

## Agent inventory

### Mindset agents (1:1 mapping)

Each of the 7 mindsets gets a corresponding agent:

| Mindset | Agent | Tools | Purpose |
|---------|-------|-------|---------|
| Analyst | `analyst.md` | Read, Grep, Glob | Autonomous problem analysis before design |
| Developer | `developer.md` | Read, Grep, Glob, Bash | Verify implementation approach before coding |
| Acceptance Tester | `acceptance-tester.md` | Read, Grep, Glob, Bash | Autonomous acceptance testing from user perspective |
| UX Designer | `ux-designer.md` | Read, Grep, Glob | Review interfaces for usability, accessibility, consistency |
| Security Reviewer | `security-reviewer.md` | Read, Grep, Glob, Bash | Audit for OWASP, auth, data exposure, injection |
| Performance Engineer | `performance-engineer.md` | Read, Grep, Glob, Bash | Analyze complexity, memory, network, bottlenecks |
| DevOps/SRE | `devops-sre.md` | Read, Grep, Glob, Bash | Check deployability, monitoring, rollback, failure modes |

### Generic persona agent

One agent that loads **any** persona from `.sensei/personas/` to perform tasks from that persona's perspective:

| Agent | Tools | Purpose |
|-------|-------|---------|
| `persona-reviewer.md` | Read, Grep, Glob | Review work from one or all personas' perspective |

Usage:
- `@persona-reviewer check the new API endpoint` — reviews from all personas
- `@persona-reviewer as API Consumer, is this endpoint intuitive?` — one persona

This eliminates the need for a dedicated agent per persona. The generic agent handles ad-hoc persona reviews.

## Agent structure

```
marketplace/
  mindsets/              ← why + what (passive)
    analyst.md
    developer.md
    acceptance-tester.md
    ux-designer.md
    security-reviewer.md
    performance-engineer.md
    devops-sre.md
  agents/                ← why + what + HOW (active)
    analyst.md           ← wraps mindsets/analyst.md + procedures
    developer.md
    acceptance-tester.md
    ux-designer.md
    security-reviewer.md
    performance-engineer.md
    devops-sre.md
    persona-reviewer.md  ← generic, loads any persona
```

Each agent `.md` has frontmatter + two sections:

```markdown
---
name: acceptance-tester
description: Acceptance testing — verify implementation from user perspective
tools: Read, Glob, Grep, Bash
model: sonnet
---

## Mindset (what + why)

[full content from marketplace/mindsets/acceptance-tester.md — questions preserved exactly]

## Procedure (how)

1. Read `.sensei/rules.md` and `.sensei/personas/*.md`
2. Identify changed files (git diff)
3. For each mindset question, check if it was addressed
4. For each persona, walk through changes from their perspective
5. Report: questions answered ✓ / missed ✗, persona findings, action recipes
```

## How agents integrate with sensei

Agents use the same MCP tools as commands:
- `search()`, `get_callers()`, `get_callees()` for code intelligence
- Read `.sensei/rules.md` for project rules
- Read `.sensei/personas/*.md` for persona context
- Call `update_session()` for event capture

The daemon doesn't need to know about agents — they're plugin-level (marketplace). Claude Code discovers them from the plugin's `agents/` directory.

## Promotion path

Projects start with mindsets only (low overhead). When deeper autonomous verification is needed:

1. **Ship defaults** — `marketplace/agents/` ships all 7 mindset agents + persona-reviewer
2. **Project customization** — users can copy an agent to `.sensei/agents/` and customize the procedure
3. **Create from scratch** — `/sensei:agent create` or desktop Profiles page

## Open questions

| # | Question | Status |
|---|----------|--------|
| 1 | Which agent to build first? | **acceptance-tester** — highest value, catches quality issues autonomously |
| 2 | Can agents spawn sub-agents? | Defer — start simple, add composition later |
| 3 | How do agents report back? | Summary to main conversation, full findings available on request |
| 4 | Worktrees or same directory? | Same directory — agents are read-heavy, not write-heavy |
| 5 | Testing approach? | Skill-creator evals — same framework, agents are just autonomous skills |
