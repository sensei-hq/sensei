---
type: design
---
# Design

Purpose: the design system + cross-cutting UX rules; audience: designer + developer.

Canonical design sources:

- [`mockup-brief.md`](mockup-brief.md) — how we design mockups: the loop, the guardrails,
  the component vocabulary, and the designer task template. The live review of outstanding
  screens is [`../mockups/dojo2-review.md`](../mockups/dojo2-review.md).
- [`../architecture/frontend-svelte-guidelines.md`](../architecture/frontend-svelte-guidelines.md) —
  the enforced frontend rules: the 24 design tokens, the type scale, and spacing.
- [`../mockups/Sensei/CLAUDE.md`](../mockups/Sensei/CLAUDE.md) — the same system for
  claude.ai artifacts (the `.zs` no-rokkit drop-in) plus the pre-delivery self-check.

## Modules

Cross-feature, behind-the-scenes design (the how, paired with a feature's what).
Each names the code that realises it.

- [`setup-and-config.md`](setup-and-config.md) — **bootstrap**: the [Setup](../features/01-setup.md) entry gate + [Configuration](../features/02-config.md) surface (health gate, install, folder scan, config surface).
- [`api-daemon.md`](api-daemon.md) — the senseid daemon + HTTP API (:7744), gateway routing (local vs cloud), the core loop, Postgres + pgvector, SSE.
- [`workers.md`](workers.md) — scan · incremental watcher · reconcile · analyzer scheduler + passes · background-task visibility.
- [`assistants.md`](assistants.md) — ACP adapters, MCP context delivery, hooks (capture + relay control channel), marketplace packaging.
- [`projects.md`](projects.md) — folders → repos → projects/solutions, the code + activity graph, the project window.
- [`playbook.md`](playbook.md) — the front-door intake mechanics (classify → recommend → confirm), axes→playbook matrix, learning loop, `playbook_run` contract. Supports [Project · Working style](../features/04-project.md#working-style--the-operation-manual).
- [`remote.md`](remote.md) — the relay run-engine + hook control-channel + gates/nudges, and the future Planner→Builder→Judge orchestrator (Phase 6).
- [`governance.md`](governance.md) — dōjō scopes + precedence, rules resolution, promotion, identity/confidentiality, budgets & controls, local-model inclusion.
- [`instruction-delivery-model.md`](instruction-delivery-model.md) — how rules + capabilities (skills/agents/mindsets) reach and *stick* in a session: the delivery surfaces, the stickiness problem, and the enforcement-tiered push model + rule/rule-pack shape.
- [`library-auto-discovery.md`](library-auto-discovery.md) — **auto-index detected dependencies**: when `extract_deps` finds a library in a manifest, auto-discover its llms.txt + skills/agents + MCP tools and make them available to the AI without manual `add_library` calls. Phase 1 story P1.7/P1.8.
- [`phases.md`](phases.md) — **incremental delivery phases** (the strategic sequencing). Four phases, each shipping value independently: context pushes itself → Dōjō join surface → Relay for one → governance plane. Read this first for the "what ships when and why"; the dated build plans in `plan/` are the execution layer within each phase.

**Build order (2026-07-20): analysis first.** The *synthetic analysis*
capabilities (governance, metrics, document drift, graph) are the foundation —
they produce the guiding principles. **Recommendations & guidance come later**,
once the analysis reports give something to guide with. MVP grouping + phases
are decided together, after every bullet is written up.


## Layers

The layered architecture involves the following

- Database: PostgreSQL database managed using [dbd](https://dbd.sensei-hq.com) a rust cli for managing database setup, migrations and deployments. [llms for dbd](https://dbd.sensei-hq.com/llms.txt)
- Daemon: An always running daemon that receives data from assistants and a wrapper on the database
- cli: a cli wrapper for accessing the tooling without using the desktop app
- mcp: a toolkit exposing sensei data (memory, guidance, context) to make it availabel to llm assistants
- Sensei: A Desktop app for observiing and interfacing with user's projects
- Dojo: A web saas provisioned system which provides a control plane for all users accessing code via sensei
