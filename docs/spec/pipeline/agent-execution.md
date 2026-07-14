# 使 · Pipeline · Agent execution

**Owner files:**
- Agent registry: `marketplace/plugins/sensei/agents/*.md`
- Runtime: `crates/senseid/src/agents/runtime.rs`
- Isolation: worktree option per `Agent(subagent_type=…, isolation=…)`
- Reporting: `crates/senseid/src/agents/report.rs`
- MCP dispatch: `crates/mcp/src/tools/agent_run.rs`

**Companion design doc:** [`docs/archive/ideas/21-custom-agents.md`](../../archive/ideas/21-custom-agents.md).

## Purpose

Agents are focused specialists sensei can dispatch for a defined
task. Each agent wraps a mindset (why + what → how):

- **Why** — the perspective the agent brings (e.g. security
  reviewer looks for OWASP top-10 + auth boundary leaks).
- **What** — the deliverable (a review report, a spec doc, a
  verification result).
- **How** — the procedure the agent runs (which tools it reaches
  for, in what order, what evidence it collects).

Agents run in **isolated context** — they don't see the calling
session's history and their only output is their final report.
This keeps their reasoning uncontaminated and their output
composable.

Kanji is 使 — *envoy / to dispatch*.

## Data invariants

### Agent definition

Each agent is a `.md` file with YAML frontmatter:

    ---
    name: sensei-security-reviewer
    description: |
      Audit code for security vulnerabilities including OWASP top 10, auth
      issues, data exposure, and injection vectors. Use proactively when a task
      involves user input, authentication, data storage, or external
      communication.
    tools: Read, Grep, Glob, Bash, mcp__plugin_sensei_sensei__*
    model: sonnet
    color: red
    ---

    ## Purpose
    …
    ## Procedure
    …
    ## Report format
    …

- `name` — global unique id.
- `description` — includes examples of when to use / when NOT to;
  read by the calling model to decide when to dispatch.
- `tools` — the allowed-tool list. Agents cannot exceed their
  declared surface (verifier gate at dispatch time).
- `model` — model family recommendation; overridable by the caller.
- Body has three sections: Purpose (why), Procedure (how), Report
  format (what).

### The default agent set

Living under `marketplace/plugins/sensei/agents/`:

| Agent | Purpose |
|---|---|
| `sensei-analyst` | Requirements clarity before design |
| `sensei-developer` | Approach validation before coding |
| `sensei-acceptance-tester` | End-to-end journey verification |
| `sensei-ux-designer` | Interface / voice review |
| `sensei-security-reviewer` | Security audit |
| `sensei-performance-engineer` | Complexity + scalability review |
| `sensei-devops-sre` | Deployability + reliability review |
| `sensei-persona-reviewer` | Persona-perspective verification |

Additional gate agents live under `.claude/agents/` (project-
local):

| Agent | Purpose |
|---|---|
| `spec-doc-reviewer` | Reviews an spec doc for completeness |
| `done-gate-verifier` | Executes the done gate against the daemon |
| `wrong-gate-hunter` | Actively probes for anti-patterns |

### Isolation

Two modes:

- **In-place** — runs in the current working directory. Fine for
  read-only agents (reviewers, verifiers).
- **Worktree** — creates a git worktree so the agent can modify
  code without stepping on the caller's changes. Used by
  autonomous builders during vacation runs, `/loop`, etc.
  Automatic cleanup if the agent makes no changes; otherwise
  the caller gets the path + branch name for merge.

### Tool restriction (verifier gate)

At dispatch, the runtime checks that the agent's requested tools
are a subset of its declared `tools` list. Attempts to use a
disallowed tool return an error to the agent (with the specific
tool named), giving it a chance to fall back or report the
inability.

### Reporting

Every agent returns a single message when it exits — that's the
entire return value. Convention (see `.claude/agents/*` for
examples):

    # {AgentName}: {target}

    **Verdict:** ready | needs-fixes | not-ready | pass | fail

    ## Pass
    - …

    ## Fail
    - …

    ## Recommendations
    - …

Stored in `sensei.agent_runs` with the invocation context
(caller session, agent name, target, verdict, report body,
duration_ms).

## Signals produced

| Signal | Consumer |
|---|---|
| Agent reports | Caller session |
| `sensei.agent_runs` | Telemetry for agent effectiveness |
| Repeated `not-ready` verdicts on a given target | Insights hint ("this spec keeps failing review") |
| Repeated success | Confidence signal — the caller can trust the pattern |

## Done gate

- Every default agent is dispatchable and returns a report in
  the standard shape.
- Tool restriction is enforced (attempts to use disallowed
  tools error, not silently work).
- Worktree isolation cleans up automatically for read-only
  agents; leaves the worktree in place for agents that made
  changes.
- Agent runs persist to `sensei.agent_runs` with a queryable
  history.
- The gate agents (spec-doc-reviewer / done-gate-verifier /
  wrong-gate-hunter) integrate with the [[agents/README]]
  playbook.

Optional check:
```
# Which agents ran in the last hour?
psql -A -t -c "select agent, verdict, count(*) from sensei.agent_runs
                where ran_at > now() - interval '1 hour'
                group by agent, verdict" -d sensei
```

## Wrong gate

- **An agent uses a tool it didn't declare.** Tool restriction
  gate bypassed.
- **Agent report leaks caller session context.** Isolation
  broken — agent should see only its explicit input.
- **Worktree agent leaves stale worktrees behind.** Cleanup
  path missed.
- **Report format wildly varies between agents of the same
  class.** Convention needs enforcement (add a small
  spec-doc-reviewer-style check per agent).
- **Multiple agents of the same purpose exist across the
  marketplace with subtly different behaviour.** Rationalise
  or clearly differentiate.

## Related

- [[agents/README]] — the gate playbook (spec review / done /
  wrong-gate)
- [[pipeline/inferencing]] — model routing for agents
- [[pipeline/governance]] — verifier + approver pattern is
  analogous
- [[pipeline/mcp-surface]] — tool declarations that agents draw
  from
- (memory: project_vacation_run_2026_07) (memory) — gated per-doc
  execution recipe uses the agent set
- (archive: ideas/21-custom-agents.md) — source design
