# Mindsets

> A mindset is a thinking lens the AI puts on to work from a particular role's perspective. See also [personas](./personas.md) and [agents](./agents.md).

## What a mindset is

A mindset is a Claude Code subagent that "wears a hat" — it adopts a role, asks that role's questions, and performs the work from that perspective. "Think like an analyst before you design." "Think like a security reviewer before you ship the auth endpoint." Each mindset is a small, focused discipline: a set of questions plus a procedure for answering them and a report format.

Mindsets are **mechanism-identical to agents** — every mindset *is* an agent file. They live in the plugin at `marketplace/plugins/sensei/agents/*.md`. There is no separate `mindsets/` or `templates/` folder; the agent file is the single source of truth. When this doc says "the mindset," it means the same `.md` file the [agents](./agents.md) doc calls "the agent." The two words describe the same artifact from two angles: *mindset* is the thinking lens it encodes, *agent* is the subagent that runs it.

Mindsets answer **how to approach the work**. That distinguishes them from [personas](./personas.md), which answer **who the work is for**.

## The core three, in sequence

Three mindsets apply to every task, in order:

```
Analyst  →  Developer  →  Acceptance Tester
```

| # | Mindset | Question it forces | Agent file |
|---|---------|--------------------|------------|
| 1 | **Analyst** | Do we understand the *problem* before designing? | `agents/analyst.md` |
| 2 | **Developer** | Do we understand the *implementation* before coding? | `agents/developer.md` |
| 3 | **Acceptance Tester** | Does the result *deliver value* to the user — demonstrably, not "probably"? | `agents/acceptance-tester.md` |

A sketch of what each asks:

- **Analyst** — What problem are we solving, in the user's words? Who benefits? What are the constraints, the acceptance criteria, the edge cases? What are we explicitly *not* building? If requirements are unclear, surface the ambiguity — don't fill gaps with assumptions.
- **Developer** — Where does this run, and who reads it? How does it get there? What happens when it's missing? How do I verify it works? Every file has to justify its existence; a question costs one turn, a wrong assumption costs a rewrite.
- **Acceptance Tester** — Walk the user journey end to end. Test the happy path, the first-time experience, the failure path, the correction path. Verify each acceptance criterion with evidence. Check for regressions. "Probably works" is not verification.

### Why sequencing matters

The order is the point. Skipping analysis means designing for a misunderstood problem. Skipping the developer lens means coding before knowing where the code runs or whether it duplicates something that exists. Skipping the tester lens means declaring "done" without demonstrating it.

Most AI coding failures are *premature* — the model jumps to code before it understands the problem or the codebase, and you spend the next several turns correcting it. Running the lenses in sequence front-loads the understanding so the first attempt is the right one. That is the whole bet: structured thinking now, fewer corrections later. (See [FTR](#why-mindsets-tie-to-ftr).)

## Specialist mindsets

Four more mindsets apply **only when their domain is relevant** — you don't run a performance pass on a copy-edit. They are enabled per project and invoked on demand.

| Mindset | Apply when | Agent file |
|---------|-----------|------------|
| **UX Designer** | Commands, UI components, output formatting, user-facing messages | `agents/ux-designer.md` |
| **Security Reviewer** | User input, authentication, data storage, external communication | `agents/security-reviewer.md` |
| **Performance Engineer** | Data processing, queries, loops, user-facing latency | `agents/performance-engineer.md` |
| **DevOps / SRE** | Deployment, infrastructure, configuration, reliability-sensitive changes | `agents/devops-sre.md` |

Each carries the same shape as the core three — a short *what + why*, a list of questions, a procedure, and a report format. For example, the Security Reviewer assumes adversarial input on every boundary and asks "what can go wrong?"; the Performance Engineer asks "what's the cost, can it handle 10x?"; DevOps/SRE asks "can this be deployed, monitored, rolled back — what breaks at 3am?"

There is also a **persona-reviewer**, which is *not* a fixed mindset — it's a generic agent that loads your project [personas](./personas.md) and validates against them. It belongs to the persona world, not the mindset world, but it ships alongside the mindset agents.

## How each mindset maps to an agent file

Every mindset agent file has the same anatomy:

```markdown
---
name: sensei-analyst
description: Autonomous problem analysis before designing or building...
tools: Read, Grep, Glob
model: sonnet
color: blue
---

## Mindset (what + why)      ← the lens: a principle + numbered Questions
## Procedure (how)            ← steps the agent runs when invoked
## Report Format              ← the structured output it returns
```

- **Mindset (what + why)** is the thinking lens — the questions, preserved as the heart of the agent.
- **Procedure (how)** is what makes it an *agent*: concrete steps (read `.sensei/rules.md`, read `.sensei/personas/*.md`, run `git diff`, search the code graph, answer each question with evidence).
- **Report Format** is the structured verdict — a table of criteria met/missed, findings, recommendations.

So the same file is "a mindset" (the lens) and "an agent" (the autonomous runner). See [agents.md](./agents.md) for the frontmatter fields, tool scoping, and how custom agents are defined.

## How mindsets are applied

There are two complementary modes:

1. **Lean reminder at session start.** When a session begins, Sensei injects a compact reminder into context: *apply the core three in sequence (Analyst → Developer → Acceptance Tester), and apply specialists when their domain applies.* This is cheap — a few lines — and keeps the disciplines top-of-mind without spawning anything. The AI internalizes the questions and works through them inline.

2. **Invoked as subagents on demand.** When you want a focused, autonomous pass, you dispatch the mindset as a subagent — for example via `/sensei:agent use sensei-security-reviewer review the new auth endpoint`. The agent runs in isolated context with its scoped tools, executes its procedure, and returns its structured report. This is the heavyweight mode: deep, independent analysis that doesn't clutter the main conversation.

The reminder keeps every session honest; the subagent gives you an expert deep-dive when a domain warrants it.

## Why mindsets tie to FTR

Sensei's hero metric is **FTR (First-Time-Right)** — the share of sessions where the assistant produces correct code with no corrections. Mindsets are the primary lever:

- **They prevent premature code.** Analyst-before-developer-before-tester means the model resolves ambiguity and understands the codebase *before* it writes, instead of being corrected into understanding it.
- **They make "done" mean done.** The acceptance-tester lens requires demonstrated criteria, so work isn't declared finished while a user journey is still broken.
- **They surface domain risk early.** A security or performance pass at the right moment catches a class of correction that would otherwise show up in review — or in production.

Fewer corrections is exactly what FTR measures. If a mindset is active but sessions still fail in its area, that's a signal the questions need strengthening — not that the mindset failed.

---

**Related:** [personas.md](./personas.md) · [agents.md](./agents.md) · [concepts/README.md](./README.md)
