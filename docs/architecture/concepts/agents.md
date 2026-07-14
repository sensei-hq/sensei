# Agents

> An agent is the Claude Code subagent mechanism that *implements* mindsets and runs autonomous, multi-step tasks. See also [mindsets](./mindsets.md) and [personas](./personas.md).

## What an agent is

An **agent** is a Claude Code subagent: a unit that runs in its own isolated context, with its own restricted set of tools, executes a procedure autonomously, and returns a structured report to the main conversation. It's the *engine* — the construct that actually does work on its own.

Agents are the layer underneath the other two concepts:

- **Mindsets** are *realized as* agents. Each mindset ships as one agent file (analyst, developer, tester, and the specialists). The agent is what turns a thinking lens into something that can run.
- **Personas** are *served by* an agent — the single generic `persona-reviewer`, which loads any [persona](./personas.md) you've defined and validates work from its perspective.

A useful way to keep the three straight:

| Term | Is | Lives in |
|------|----|----------|
| **Mindset** | a thinking lens, shipped *as* an agent | `marketplace/plugins/sensei/agents/*.md` |
| **Persona** | a project-defined user archetype to validate against | `.sensei/personas/*.md` |
| **Agent** | the subagent construct that runs mindsets and persona reviews | `marketplace/plugins/sensei/agents/*.md` (built-in) or `.sensei/agents/*.md` (custom) |

## Why agents exist

Mindsets tell the AI *what* to think about and *why* — but a bare reminder in session context is passive. Slash commands are single-turn. For deeper work — an acceptance pass across every persona, a full OWASP audit, a code review against all patterns — you want something that can run *autonomously*, in *isolated context* (so it doesn't pollute the main conversation), with *restricted tools* (so a review agent can't accidentally write), and return a *clean report*. That's an agent.

So the agent adds the execution layer on top of a mindset:

| Layer | Contains | Execution |
|-------|----------|-----------|
| **Mindset** (what + why) | Questions to ask, principles to follow | Passive — a reminder in session context |
| **Agent** (what + why + *how*) | The same questions **plus** a procedure, scoped tools, and a report format | Active — runs autonomously in isolated context |

The agent *includes* the full mindset — it doesn't replace it.

## What ships: the built-in agents

The Sensei plugin ships eight agents in `marketplace/plugins/sensei/agents/`:

| Agent | Kind | Tools | Purpose |
|-------|------|-------|---------|
| `sensei-analyst` | mindset | Read, Grep, Glob + sensei MCP | Problem analysis before design |
| `sensei-developer` | mindset | Read, Grep, Glob, Bash + sensei MCP | Verify implementation approach before coding |
| `sensei-acceptance-tester` | mindset | Read, Grep, Glob, Bash + sensei MCP | Acceptance testing from the user's perspective |
| `sensei-ux-designer` | mindset | Read, Grep, Glob + sensei MCP | Usability, accessibility, consistency review |
| `sensei-security-reviewer` | mindset | Read, Grep, Glob, Bash + sensei MCP | OWASP / auth / data-exposure / injection audit |
| `sensei-performance-engineer` | mindset | Read, Grep, Glob, Bash + sensei MCP | Complexity, memory, network, scalability analysis |
| `sensei-devops-sre` | mindset | Read, Grep, Glob, Bash + sensei MCP | Deployability, monitoring, rollback, failure modes |
| `sensei-persona-reviewer` | persona | Read, Grep, Glob + sensei MCP | Review work from each project [persona's](./personas.md) perspective |

The first seven are the [mindset](./mindsets.md) agents (the core three plus four specialists). The eighth is generic: instead of fixed questions it loads whatever personas the project defines. That one generic agent eliminates the need for a dedicated agent per persona.

## How an agent is defined

Every agent is a markdown file with YAML frontmatter and a body:

```markdown
---
name: sensei-security-reviewer
description: Audit code for security vulnerabilities including OWASP top 10,
  auth issues, data exposure, and injection vectors. Use proactively when a
  task involves user input, authentication, data storage, or external comms.
tools: Read, Grep, Glob, Bash, mcp__plugin_sensei_sensei__*
model: sonnet
color: red
---

## Mindset (what + why)   ← principle + numbered Questions (the lens)
## Procedure (how)         ← steps the agent runs when invoked
## Report Format           ← the structured output it returns
```

### Frontmatter fields

| Field | Meaning |
|-------|---------|
| `name` | The agent's identifier. Built-in agents use the `sensei-` prefix (e.g. `sensei-analyst`). |
| `description` | When to use it — written so Claude Code can decide to invoke it *proactively*. "Use proactively when…" phrasing matters. |
| `tools` | The tools the agent may call — its **scope**. A review agent gets read-only tools; one that runs tests also gets `Bash`. All built-ins also carry the sensei MCP grant `mcp__plugin_sensei_sensei__*` for code-graph access. |
| `model` | Which model runs the agent (e.g. `sonnet`). |
| `color` | Display color in the UI. |

### Tool scoping

Tool scope is a guardrail, not a formality. Analysis-only agents (analyst, ux-designer, persona-reviewer) get `Read, Grep, Glob` (no `Bash`) — they read and report, they never write. Agents that must run a test suite or inspect git state (developer, acceptance-tester, security, performance, devops) add `Bash`. The scope keeps an autonomous agent inside its lane: a UX review can't mutate code, and a persona review can't run arbitrary commands.

All eight also receive the sensei MCP tools via the `mcp__plugin_sensei_sensei__*` grant. These are read-only code-graph and context queries (`search`, `get_callers`, `get_patterns`, `get_layered_context`, …) — they add **no** `Write`/`Edit`, so the read-and-report posture is preserved while the agent gains graph-aware navigation. The grant token is fixed by the plugin manifest (`mcp__plugin_<plugin>_<server>__*` → `plugin_sensei_sensei`), so it resolves identically on every install.

### Autonomy

Agents are read-heavy and run autonomously in the same working directory (no worktree needed). Once invoked, an agent works through its procedure without further prompting and returns a summary to the main conversation, with the full findings available on request. The desktop **Agent editor** ([Extensions tab](../../archive/ideas/04-project.md#extensions)) also exposes an autonomy level — fully autonomous, checkpoint-based (approval at checkpoints), or manual — and lets you test an agent against historical session replays before enabling it.

## How agents integrate with Sensei

Every built-in agent is granted the sensei MCP server and is instructed to **navigate with it first** — the code graph for structure, `Grep`/`Glob` only for literal text scans (a specific token or secret) or when the daemon is unreachable. Agents use the same Sensei MCP tools as the rest of the workflow:

- Code intelligence: `search()`, `get_callers()`, `get_callees()`, `get_patterns()`, `get_duplicates()`, `get_communities()` — graph-aware navigation instead of raw grep.
- Project context: `get_layered_context()` for rules, conventions, and learnings; `.sensei/personas/*.md` for personas.
- Telemetry: `log_event()` / `update_session()` for event capture.

The daemon doesn't need to know agents exist — they are **plugin-level**. Claude Code discovers them from the plugin's `agents/` directory.

## How mindsets and personas are realized as agents

This is the crux that ties the three concepts together:

- **A mindset becomes an agent** by being authored as one. The mindset *is* the `## Mindset (what + why)` section; the agent adds `## Procedure (how)` and `## Report Format`. Same file, two roles. There is no separate mindsets store — the agent file is the single source of truth. (See [mindsets.md](./mindsets.md).)
- **A persona becomes actionable through an agent** — the generic `sensei-persona-reviewer`. Personas themselves are just data (`.sensei/personas/*.md`); the persona-reviewer agent is the runtime that loads that data and validates against it. (See [personas.md](./personas.md).)

## Invoking and creating agents

**Invoke** via the `/sensei:agent` command:

```
/sensei:agent list                                      # show all agents
/sensei:agent use sensei-security-reviewer review the auth endpoint
/sensei:agent use sensei-persona-reviewer               # all personas
/sensei:agent use sensei-acceptance-tester verify #42
```

The command dispatches the named subagent; when it finishes, its report is surfaced directly.

**Create custom agents** when the built-ins aren't enough:

1. **Ship defaults** — the plugin ships the eight agents above.
2. **Project customization** — copy a built-in agent into `.sensei/agents/<name>.md` and adjust its procedure for your project.
3. **From scratch** — author a new `.sensei/agents/<name>.md` (or use the desktop Agent editor) with its own frontmatter, procedure, and report format.

## Why agents tie to FTR

Sensei's hero metric is **FTR (First-Time-Right)**. Agents raise it by making the [mindset](./mindsets.md) and [persona](./personas.md) disciplines *enforceable on demand*: a passive reminder is easy to skip, but an autonomous acceptance-tester or security agent actually walks the journey, checks every criterion with evidence, and reports what's missing — catching the corrections that would otherwise land on you. Isolated context and scoped tools make that deep pass cheap to run and safe to trust.

---

**Related:** [mindsets.md](./mindsets.md) · [personas.md](./personas.md) · [concepts/README.md](./README.md)
