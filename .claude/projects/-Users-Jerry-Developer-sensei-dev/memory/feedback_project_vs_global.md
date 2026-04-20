---
name: Observatory must separate project vs global scope
description: Desktop pages must distinguish project-specific data (profiles, sessions, metrics, rules) from global data (skills, plugins, MCP config, libraries, benchmarks)
type: feedback
---

The observatory currently renders everything on flat pages without scope awareness. This is wrong — an AI Driven Developer manages multiple projects.

**Project-scoped (per repo/solution):**
- Sessions and session metrics (FTR, turns, rework)
- Profiles (mindsets, personas, rules) — each project has its own .sensei/
- Code intelligence (graph, complexity, duplicates, dead code, doc drift)
- Project-specific skills

**Global (across all projects):**
- Skills catalog (installed globally, available to all projects)
- Plugins and MCP configuration
- Library docs (indexed once, shared across projects)
- Benchmarks (cross-project comparison)
- ACP configuration
- Tool catalog (MCP tools are global)

**Why:** A user with 5 projects needs to see "how is project X doing?" not "how is everything doing?" The dashboard should default to the active project, with a way to switch. Global pages (skills, plugins, libraries) are separate.

**How to apply:** Every page needs a scope — either project-scoped (with project selector) or global. The sidebar navigation should reflect this hierarchy.
