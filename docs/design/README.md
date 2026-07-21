---
type: design
---
# Design

Purpose: the design system + cross-cutting UX rules; audience: designer + developer.

This folder does not yet hold consolidated design-system content. Until that
consolidation happens (incremental), use the canonical sources directly:

- [`../architecture/frontend-svelte-guidelines.md`](../architecture/frontend-svelte-guidelines.md) —
  the enforced frontend rules: the 24 design tokens, the type scale, and spacing.
- [`../mockups/STYLING.md`](../mockups/STYLING.md) — the mockup styling drop-in.

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
